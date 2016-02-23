// Cymbalum, an extensible molecular simulation engine
// Copyright (C) 2015-2016 G. Fraux — BSD license

//! Testing physical properties of a Lennard-Jones gaz of Helium using
//! Monte-Carlo simulation
extern crate cymbalum;
use self::cymbalum::*;

use std::sync::{Once, ONCE_INIT};
static START: Once = ONCE_INIT;

use std::path::Path;

fn get_system() -> System {
    let data_dir = Path::new(file!()).parent().unwrap();
    let configuration = data_dir.join("data").join("helium.xyz");
    let mut system = System::from_file(configuration.to_str().unwrap()).unwrap();
    system.set_cell(UnitCell::cubic(10.0));

    system.add_pair_interaction("He", "He",
        Box::new(LennardJones{
            sigma: units::from(2.0, "A").unwrap(),
            epsilon: units::from(0.2, "kJ/mol").unwrap()
        })
    );
    return system;
}

#[test]
fn perfect_gaz() {
    START.call_once(|| {Logger::stdout();});
    let mut system = get_system();
    let mut mc = MonteCarlo::new(units::from(300.0, "K").unwrap());
    mc.add(Box::new(Translate::new(units::from(3.0, "A").unwrap())), 1.0);
    let mut simulation = Simulation::new(mc);

    // dilating the system!
    for particle in system.iter_mut() {
        particle.position = 10.0 * particle.position;
    }
    system.set_cell(UnitCell::cubic(100.0));

    simulation.run(&mut system, 5000);
    let pressure = system.pressure();
    let volume = system.volume();
    let temperature = system.temperature();
    let n = system.size() as f64;

    assert!(f64::abs(pressure * volume - n * constants::K_BOLTZMANN * temperature) < 1e-3);
}