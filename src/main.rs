use std::io;
use tokio_i3ipc::{
    event::{self, Event, Subscribe, WindowData},
    I3,
};
use tokio_stream::StreamExt;

use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::process::Command;

fn on_roon() {
    // We found out we were focused on Roon, lift the CPU limitation
    println!("Focused on Roon");
}

fn not_on_roon(system: &mut System, started_subprocess: &mut bool) {
    // refresh the list of processes
    system.refresh_processes_specifics(ProcessRefreshKind::new());
    let mut pid = Pid::from(0);
    for process in system.processes_by_exact_name("Roon.exe") {
        pid = process.pid();
    }

    // Don't do anything if we did not find Roon UI
    if pid == Pid::from(0) {
        return;
    }
    if !(*started_subprocess) {
        println!("Calling cpulimit on PID {:?}", pid);
        let cpulimit_process = Command::new("cpulimit")
            .arg("-p")
            .arg(pid.to_string())
            .arg("-l")
            .arg("15")
            .spawn()
            .expect("cpulimit command failed to start");
        *started_subprocess = true;
    } else {
        println!("cpulimit already running, doing nothing");
    }
}

fn on_window(event: Box<WindowData>, system: &mut System, started_subprocess: &mut bool) {
    // println!("{:?}", event);

    if event.change == event::WindowChange::Focus {
        let window_prop = match event.container.window_properties {
            Some(w) => w,
            _ => return,
        };
        //println!("{:?}", window_prop);

        let class = window_prop.class.unwrap_or("".to_string());
        if class == "roon.exe" {
            on_roon();
        } else {
            not_on_roon(system, started_subprocess);
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut started_subprocess = false;

    let mut system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );

    let mut i3 = I3::connect().await?;
    let resp = i3.subscribe([Subscribe::Window]).await?;
    println!("{:#?}", resp);

    let mut listener = i3.listen();
    while let Some(event) = listener.next().await {
        match event? {
            Event::Window(ev) => on_window(ev, &mut system, &mut started_subprocess),
            // ignore all other events
            _ => continue,
        }
    }
    Ok(())
}
