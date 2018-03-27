// Lumol, an extensible molecular simulation engine
// Copyright (C) Lumol's contributors — BSD license

#[macro_use]
extern crate bencher;
extern crate lumol;
extern crate lumol_input;
extern crate rand;

#[macro_use]
mod utils;

mod ewald {
    use bencher::Bencher;
    use rand::Rng;

    use lumol::energy::{CoulombicPotential, Ewald, GlobalPotential, PairRestriction, SharedEwald};
    use lumol::sys::EnergyCache;
    use lumol::types::{Vector3D, Zero};

    use utils;

    fn get_ewald() -> SharedEwald {
        let mut ewald = SharedEwald::new(Ewald::new(8.0, 7));
        ewald.set_restriction(PairRestriction::InterMolecular);
        ewald
    }


    pub fn energy(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let ewald = get_ewald();
        ewald.energy(&system);

        bencher.iter(|| {
            let _ = ewald.energy(&system);
        })
    }

    pub fn forces(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let ewald = get_ewald();
        let mut forces = vec![Vector3D::zero(); system.size()];
        ewald.forces(&system, &mut forces);

        bencher.iter(|| {
            ewald.forces(&system, &mut forces);
        })
    }

    pub fn virial(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let ewald = get_ewald();
        ewald.virial(&system);

        bencher.iter(|| {
            let _ = ewald.virial(&system);
        })
    }

    pub fn cache_move_particles(bencher: &mut Bencher) {
        let mut system = utils::get_system("water");
        system.set_coulomb_potential(Box::new(get_ewald()));

        let mut cache = EnergyCache::new();
        cache.init(&system);

        let mut rng = utils::get_rng(9886565);

        let molecule = rng.choose(system.molecules()).unwrap();
        let indexes = molecule.into_iter();
        let mut delta = vec![];
        for position in &system.particles().position[indexes] {
            delta.push(position + Vector3D::new(rng.gen(), rng.gen(), rng.gen()));
        }

        cache.move_particles_cost(&system, molecule.iter().collect(), &delta);

        bencher.iter(|| cache.move_particles_cost(&system, molecule.iter().collect(), &delta))
    }

    pub fn cache_move_all_rigid_molecules(bencher: &mut Bencher) {
        let mut system = utils::get_system("water");
        system.set_coulomb_potential(Box::new(get_ewald()));

        let mut cache = EnergyCache::new();
        cache.init(&system);

        let mut rng = utils::get_rng(2121);
        for molecule in system.molecules().to_owned() {
            let delta = Vector3D::new(rng.gen(), rng.gen(), rng.gen());
            let indexes = molecule.into_iter();
            for position in &mut system.particles_mut().position[indexes] {
                *position += delta;
            }
        }

        cache.move_all_rigid_molecules_cost(&system);

        bencher.iter(|| cache.move_all_rigid_molecules_cost(&system))
    }
}

mod wolf {
    use bencher::Bencher;
    use rand::Rng;

    use lumol::energy::{CoulombicPotential, GlobalPotential, PairRestriction, Wolf};
    use lumol::sys::EnergyCache;
    use lumol::types::{Vector3D, Zero};

    use utils;

    fn get_wolf() -> Wolf {
        let mut wolf = Wolf::new(9.0);
        wolf.set_restriction(PairRestriction::InterMolecular);
        wolf
    }

    pub fn energy(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let wolf = get_wolf();
        wolf.energy(&system);

        bencher.iter(|| {
            let _ = wolf.energy(&system);
        })
    }

    pub fn forces(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let wolf = get_wolf();
        let mut forces = vec![Vector3D::zero(); system.size()];
        wolf.forces(&system, &mut forces);

        bencher.iter(|| {
            wolf.forces(&system, &mut forces);
        })
    }

    pub fn virial(bencher: &mut Bencher) {
        let system = utils::get_system("water");
        let wolf = get_wolf();
        wolf.virial(&system);

        bencher.iter(|| {
            let _ = wolf.virial(&system);
        })
    }

    pub fn cache_move_particles(bencher: &mut Bencher) {
        let mut system = utils::get_system("water");
        system.set_coulomb_potential(Box::new(get_wolf()));

        let mut cache = EnergyCache::new();
        cache.init(&system);

        let mut rng = utils::get_rng(454548784);

        let molecule = rng.choose(system.molecules()).unwrap();
        let indexes = molecule.into_iter();
        let mut delta = vec![];
        for position in &system.particles().position[indexes] {
            delta.push(position + Vector3D::new(rng.gen(), rng.gen(), rng.gen()));
        }

        cache.move_particles_cost(&system, molecule.iter().collect(), &delta);

        bencher.iter(|| cache.move_particles_cost(&system, molecule.iter().collect(), &delta))
    }

    pub fn cache_move_all_rigid_molecules(bencher: &mut Bencher) {
        let mut system = utils::get_system("water");
        system.set_coulomb_potential(Box::new(get_wolf()));

        let mut cache = EnergyCache::new();
        cache.init(&system);

        let mut rng = utils::get_rng(3);
        for molecule in system.molecules().to_owned() {
            let delta = Vector3D::new(rng.gen(), rng.gen(), rng.gen());
            let indexes = molecule.into_iter();
            for position in &mut system.particles_mut().position[indexes] {
                *position += delta;
            }
        }

        cache.move_all_rigid_molecules_cost(&system);

        bencher.iter(|| cache.move_all_rigid_molecules_cost(&system))
    }
}

benchmark_group!(ewald, ewald::energy, ewald::forces, ewald::virial);
benchmark_group!(wolf, wolf::energy, wolf::forces, wolf::virial);
benchmark_group!(monte_carlo_cache,
    ewald::cache_move_particles,
    ewald::cache_move_all_rigid_molecules,
    wolf::cache_move_particles,
    wolf::cache_move_all_rigid_molecules
);

benchmark_main!(ewald, wolf, monte_carlo_cache);
