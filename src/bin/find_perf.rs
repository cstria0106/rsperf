use sysinfo::{ProcessExt, System, SystemExt};

fn main() {
    let mut system = System::new();
    system.refresh_all();
    for p in system.processes_by_exact_name("perf") {
        println!("{}", p.pid())
    }
}
