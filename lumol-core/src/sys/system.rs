// Lumol, an extensible molecular simulation engine
// Copyright (C) 2015-2016 Lumol's contributors — BSD license

use std::collections::BTreeMap;
use std::ops::{Deref, DerefMut};

use soa_derive::soa_zip;
use log_once::warn_once;

use crate::{Matrix3, Vector3D};
use crate::{AnglePotential, BondPotential, DihedralPotential, PairInteraction};
use crate::{CoulombicPotential, GlobalPotential};
use crate::{Composition, EnergyEvaluator, Interactions};
use crate::{Configuration, Molecule, ParticleKind, UnitCell};

/// The number of degrees of freedom simulated in a given system
#[derive(Clone, PartialEq, Debug)]
pub enum DegreesOfFreedom {
    /// All particles are explicitly simulated
    Particles,
    /// All molecules are simulated as rigid bodies
    Molecules,
    /// All particles are explicitly simulated, but some degrees of freedom
    /// are frozen. The usize value is the number of frozen degree of freedom.
    Frozen(usize),
}

/// The `System` type hold all the data about a simulated system.
///
/// This data contains:
///
///   - an unit cell, containing the system;
///   - a list of particles in the system;
///   - a list of molecules in the system;
///   - a list of interactions, associating particles kinds and potentials
///
/// In the implementation, the particles contained in a molecule are guaranteed
/// to be contiguous in memory. This allow for faster access when iterating
/// over molecules, and easier molecule removal from the system.
#[derive(Clone)]
pub struct System {
    /// The system configuration
    configuration: Configuration,
    /// All the interactions in this system
    interactions: Interactions,
    /// Association particles names to particle kinds
    kinds: BTreeMap<String, ParticleKind>,
    /// Externally managed temperature for the system
    external_temperature: Option<f64>,
    /// Number of degrees of freedom simulated in the system. This default to
    /// `DegreesOfFreedom::Particles`, and is set in the simulation setup.
    pub simulated_degrees_of_freedom: DegreesOfFreedom,
    /// The current simulation step
    pub step: u64,
}

impl System {
    /// Create a new empty `System`
    pub fn new() -> System {
        System::with_configuration(Configuration::new())
    }

    /// Create an empty system with a specific unit cell
    pub fn with_cell(cell: UnitCell) -> System {
        let mut configuration = Configuration::new();
        configuration.cell = cell;
        System::with_configuration(configuration)
    }

    /// Create a system with the specified `configuration`, and no interactions.
    fn with_configuration(configuration: Configuration) -> System {
        System {
            configuration: configuration,
            kinds: BTreeMap::new(),
            interactions: Interactions::new(),
            step: 0,
            external_temperature: None,
            simulated_degrees_of_freedom: DegreesOfFreedom::Particles,
        }
    }

    fn get_kind(&mut self, name: &str) -> ParticleKind {
        if let Some(&kind) = self.kinds.get(name) {
            return kind;
        } else {
            let kind = ParticleKind(self.kinds.len() as u32);
            let _ = self.kinds.insert(String::from(name), kind);
            kind
        }
    }

    /// Add a molecule to the system
    pub fn add_molecule(&mut self, mut molecule: Molecule) {
        for (kind, name) in soa_zip!(molecule.particles_mut(), [mut kind, name]) {
            *kind = self.get_kind(name);
        }
        self.configuration.add_molecule(molecule);
    }

    /// Get the composition in particles and molecules of the configuration
    pub fn composition(&self) -> Composition {
        let mut composition = Composition::new();
        for &kind in self.particles().kind {
            composition.add_particle(kind);
        }
        for molecule in self.molecules() {
            composition.add_molecule(molecule.hash());
        }
        return composition;
    }

    /// Use an external temperature for all the system properties. Calling this
    /// with `Some(temperature)` will replace all the computation of the
    /// temperature from the velocities with the given values. Calling it with
    /// `None` will use the velocities.
    ///
    /// The default is to use the velocities unless this function is called.
    pub fn simulated_temperature(&mut self, temperature: Option<f64>) {
        if let Some(temperature) = temperature {
            assert!(temperature >= 0.0, "External temperature must be positive");
        }
        self.external_temperature = temperature;
    }
}

/// Functions related to interactions
impl System {
    /// Get an helper struct to evaluate the energy of this system.
    pub fn energy_evaluator(&self) -> EnergyEvaluator<'_> {
        EnergyEvaluator::new(self)
    }

    /// Add the `potential` pair interaction for atoms with types `i` and `j`
    pub fn add_pair_potential(&mut self, (i, j): (&str, &str), potential: PairInteraction) {
        if self.cell.lengths().iter().any(|&d| 0.5 * d < potential.cutoff()) {
            panic!(
                "Can not add a potential with a cutoff bigger than half of the \
                smallest cell length. Try increasing the cell size or decreasing \
                the cutoff."
            );
        }
        let kind_i = self.get_kind(i);
        let kind_j = self.get_kind(j);
        self.interactions.add_pair((kind_i, kind_j), potential)
    }

    /// Add the `potential` bonded interaction for atoms with types `i` and `j`
    pub fn add_bond_potential(&mut self, (i, j): (&str, &str), potential: Box<dyn BondPotential>) {
        let kind_i = self.get_kind(i);
        let kind_j = self.get_kind(j);
        self.interactions.add_bond((kind_i, kind_j), potential)
    }

    /// Add the `potential` angle interaction for the angle `(i, j, k)`
    pub fn add_angle_potential(
        &mut self,
        (i, j, k): (&str, &str, &str),
        potential: Box<dyn AnglePotential>,
    ) {
        let kind_i = self.get_kind(i);
        let kind_j = self.get_kind(j);
        let kind_k = self.get_kind(k);
        self.interactions.add_angle((kind_i, kind_j, kind_k), potential)
    }

    /// Add the `potential` dihedral interaction for the dihedral angle `(i, j,
    /// k, m)`
    pub fn add_dihedral_potential(
        &mut self,
        (i, j, k, m): (&str, &str, &str, &str),
        potential: Box<dyn DihedralPotential>,
    ) {
        let kind_i = self.get_kind(i);
        let kind_j = self.get_kind(j);
        let kind_k = self.get_kind(k);
        let kind_m = self.get_kind(m);
        self.interactions.add_dihedral((kind_i, kind_j, kind_k, kind_m), potential)
    }

    /// Set the coulombic interaction for all pairs to `potential`
    pub fn set_coulomb_potential(&mut self, potential: Box<dyn CoulombicPotential>) {
        if let Some(cutoff) = potential.cutoff() {
            if self.cell.lengths().iter().any(|&d| 0.5 * d < cutoff) {
                panic!(
                    "Can not add a potential with a cutoff bigger than half of the \
                    smallest cell length. Try increasing the cell size or decreasing \
                    the cutoff."
                );
            }
        }
        self.interactions.coulomb = Some(potential);
    }

    /// Add the `potential` global interaction
    pub fn add_global_potential(&mut self, potential: Box<dyn GlobalPotential>) {
        self.interactions.globals.push(potential);
    }

    /// Get the list of pair potential acting between the particles at indexes
    /// `i` and `j`.
    pub fn pair_potentials(&self, i: usize, j: usize) -> &[PairInteraction] {
        let kind_i = self.particles().kind[i];
        let kind_j = self.particles().kind[j];
        let pairs = self.interactions.pairs((kind_i, kind_j));
        if pairs.is_empty() {
            // Use the same sorting as interactions
            let name_i = &self.particles().name[i];
            let name_j = &self.particles().name[j];
            let (name_i, name_j) = if name_i < name_j {
                (name_i, name_j)
            } else {
                (name_j, name_i)
            };
            warn_once!("No potential defined for the pair ({}, {})", name_i, name_j);
        }
        return pairs;
    }

    /// Get read-only access to the interactions for this system
    pub(crate) fn interactions(&self) -> &Interactions {
        &self.interactions
    }

    /// Get the list of bonded potential acting between the particles at indexes
    /// `i` and `j`.
    pub fn bond_potentials(&self, i: usize, j: usize) -> &[Box<dyn BondPotential>] {
        let kind_i = self.particles().kind[i];
        let kind_j = self.particles().kind[j];
        let bonds = self.interactions.bonds((kind_i, kind_j));
        if bonds.is_empty() {
            // Use the same sorting as interactions
            let name_i = &self.particles().name[i];
            let name_j = &self.particles().name[j];
            let (name_i, name_j) = if name_i < name_j {
                (name_i, name_j)
            } else {
                (name_j, name_i)
            };
            warn_once!("No potential defined for the bond ({}, {})", name_i, name_j);
        }
        return bonds;
    }

    /// Get the list of angle interaction acting between the particles at
    /// indexes `i`, `j` and `k`.
    pub fn angle_potentials(&self, i: usize, j: usize, k: usize) -> &[Box<dyn AnglePotential>] {
        let kind_i = self.particles().kind[i];
        let kind_j = self.particles().kind[j];
        let kind_k = self.particles().kind[k];
        let angles = self.interactions.angles((kind_i, kind_j, kind_k));
        if angles.is_empty() {
            // Use the same sorting as interactions
            let name_i = &self.particles().name[i];
            let name_j = &self.particles().name[j];
            let name_k = &self.particles().name[k];
            let (name_i, name_j, name_k) = if name_i < name_k {
                (name_i, name_j, name_k)
            } else {
                (name_k, name_j, name_i)
            };
            warn_once!("No potential defined for the angle ({}, {}, {})", name_i, name_j, name_k);
        }
        return angles;
    }

    /// Get the list of dihedral angles interaction acting between the particles
    /// at indexes `i`, `j`, `k` and `m`.
    pub fn dihedral_potentials(
        &self,
        i: usize,
        j: usize,
        k: usize,
        m: usize,
    ) -> &[Box<dyn DihedralPotential>] {
        let kind_i = self.particles().kind[i];
        let kind_j = self.particles().kind[j];
        let kind_k = self.particles().kind[k];
        let kind_m = self.particles().kind[m];
        let dihedrals = self.interactions.dihedrals((kind_i, kind_j, kind_k, kind_m));
        if dihedrals.is_empty() {
            // Use the same sorting as interactions
            let name_i = &self.particles().name[i];
            let name_j = &self.particles().name[j];
            let name_k = &self.particles().name[k];
            let name_m = &self.particles().name[m];
            let max_ij = ::std::cmp::max(name_i, name_j);
            let max_km = ::std::cmp::max(name_k, name_m);
            let (name_i, name_j, name_k, name_m) = if max_ij == max_km {
                if ::std::cmp::min(name_i, name_j) < ::std::cmp::min(name_k, name_m) {
                    (name_i, name_j, name_k, name_m)
                } else {
                    (name_m, name_k, name_j, name_i)
                }
            } else if max_ij < max_km {
                (name_i, name_j, name_k, name_m)
            } else {
                (name_m, name_k, name_j, name_i)
            };
            warn_once!("No potential defined for the dihedral angle ({}, {}, {}, {})", name_i, name_j, name_k, name_m);
        }
        return dihedrals;
    }

    /// Get the coulombic interaction for the system
    pub fn coulomb_potential(&self) -> Option<&dyn CoulombicPotential> {
        self.interactions.coulomb.as_ref().map(|coulomb| &**coulomb)
    }

    /// Get all global interactions for the system
    pub fn global_potentials(&self) -> &[Box<dyn GlobalPotential>] {
        &self.interactions.globals
    }

    /// Get maximum cutoff from `coulomb`, `pairs` and `global` interactions.
    pub fn maximum_cutoff(&self) -> Option<f64> {
        self.interactions.maximum_cutoff()
    }
}

use crate::compute::{KineticEnergy, PotentialEnergy, TotalEnergy};
use crate::compute::{Pressure, Stress, Virial};
use crate::compute::{PressureAtTemperature, StressAtTemperature};
use crate::compute::Compute;
use crate::compute::Forces;
use crate::compute::Temperature;
use crate::compute::Volume;

/// Functions to get physical properties of a system.
impl System {
    /// Get the number of degrees of freedom in the system
    pub fn degrees_of_freedom(&self) -> usize {
        match self.simulated_degrees_of_freedom {
            DegreesOfFreedom::Particles => 3 * self.size(),
            DegreesOfFreedom::Frozen(frozen) => 3 * self.size() - frozen,
            DegreesOfFreedom::Molecules => 3 * self.molecules().count(),
        }
    }

    /// Get the kinetic energy of the system.
    pub fn kinetic_energy(&self) -> f64 {
        KineticEnergy.compute(self)
    }

    /// Get the potential energy of the system.
    pub fn potential_energy(&self) -> f64 {
        PotentialEnergy.compute(self)
    }

    /// Get the total energy of the system.
    pub fn total_energy(&self) -> f64 {
        TotalEnergy.compute(self)
    }

    /// Get the temperature of the system.
    pub fn temperature(&self) -> f64 {
        match self.external_temperature {
            Some(value) => value,
            None => Temperature.compute(self),
        }
    }

    /// Get the volume of the system.
    pub fn volume(&self) -> f64 {
        Volume.compute(self)
    }

    /// Get the virial of the system as a tensor
    pub fn virial(&self) -> Matrix3 {
        Virial.compute(self)
    }

    /// Get the pressure of the system from the virial equation, at the system
    /// instantaneous temperature.
    pub fn pressure(&self) -> f64 {
        match self.external_temperature {
            Some(temperature) => {
                PressureAtTemperature {
                    temperature: temperature,
                }.compute(self)
            }
            None => Pressure.compute(self),
        }
    }

    /// Get the stress tensor of the system from the virial equation.
    pub fn stress(&self) -> Matrix3 {
        match self.external_temperature {
            Some(temperature) => {
                StressAtTemperature {
                    temperature: temperature,
                }.compute(self)
            }
            None => Stress.compute(self),
        }
    }

    /// Get the forces acting on all the particles in the system
    pub fn forces(&self) -> Vec<Vector3D> {
        Forces.compute(self)
    }
}

impl Deref for System {
    type Target = Configuration;

    fn deref(&self) -> &Configuration {
        &self.configuration
    }
}

impl DerefMut for System {
    fn deref_mut(&mut self) -> &mut Configuration {
        &mut self.configuration
    }
}

#[cfg(test)]
mod tests {
    use crate::{System, Molecule, Particle, ParticleKind};

    #[test]
    #[should_panic]
    fn negative_simulated_temperature() {
        let mut system = System::new();
        system.simulated_temperature(Some(-1.0));
    }

    #[test]
    fn deref() {
        let mut system = System::new();
        system.add_molecule(Molecule::new(Particle::new("H")));
        system.add_molecule(Molecule::new(Particle::new("O")));
        system.add_molecule(Molecule::new(Particle::new("H")));
        assert_eq!(system.molecules().count(), 3);

        // This uses deref_mut
        let _ = system.add_bond(0, 1);
        let _ = system.add_bond(2, 1);

        // This uses deref
        assert_eq!(system.molecules().count(), 1);
    }

    #[test]
    fn add_molecule() {
        let mut system = System::new();
        system.add_molecule(Molecule::new(Particle::new("H")));
        system.add_molecule(Molecule::new(Particle::new("O")));
        system.add_molecule(Molecule::new(Particle::new("H")));

        assert_eq!(system.particles().kind[0], ParticleKind(0));
        assert_eq!(system.particles().kind[1], ParticleKind(1));
        assert_eq!(system.particles().kind[2], ParticleKind(0));
    }

    #[test]
    fn composition() {
        let mut system = System::new();
        system.add_molecule(Molecule::new(Particle::new("H")));
        system.add_molecule(Molecule::new(Particle::new("O")));
        system.add_molecule(Molecule::new(Particle::new("O")));
        system.add_molecule(Molecule::new(Particle::new("H")));
        system.add_molecule(Molecule::new(Particle::new("C")));
        system.add_molecule(Molecule::new(Particle::new("U")));
        system.add_molecule(Molecule::new(Particle::new("H")));

        let composition = system.composition();
        assert_eq!(composition.particles(ParticleKind(0)), 3);
        assert_eq!(composition.particles(ParticleKind(1)), 2);
        assert_eq!(composition.particles(ParticleKind(2)), 1);
        assert_eq!(composition.particles(ParticleKind(3)), 1);
    }

    #[test]
    fn missing_interaction() {
        let mut system = System::new();
        system.add_molecule(Molecule::new(Particle::new("He")));
        system.add_molecule(Molecule::new(Particle::new("He")));
        system.add_molecule(Molecule::new(Particle::new("He")));
        system.add_molecule(Molecule::new(Particle::new("He")));
        assert_eq!(system.pair_potentials(0, 0).len(), 0);
        assert_eq!(system.bond_potentials(0, 0).len(), 0);
        assert_eq!(system.angle_potentials(0, 0, 0).len(), 0);
        assert_eq!(system.dihedral_potentials(0, 0, 0, 0).len(), 0);
    }
}