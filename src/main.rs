use std::io;
use tokio_i3ipc::{
    event::{self, Event, Subscribe, WindowData},
    I3,
};
use tokio_stream::StreamExt;

use sysinfo::{Pid, ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use std::process::{Child, Command};

// Embed my world in one ugly struct
struct State {
    subprocess: Option<Child>,
    system: System,
}

fn on_roon(state: &mut State) {
    // We found out we were focused on Roon, lift the CPU limitation
    println!("Focused on Roon, killing cpulimit");
    if let Some(p) = &mut state.subprocess {
        println!("Terminating cpulimit subprocess");
        let _ = p.kill();
        let _ = p.wait();
        state.subprocess = None;
    };
}

fn not_on_roon(state: &mut State) {
    // refresh the list of processes
    state
        .system
        .refresh_processes_specifics(ProcessRefreshKind::new());
    let mut pid = Pid::from(0);
    for process in state.system.processes_by_exact_name("Roon.exe") {
        pid = process.pid();
    }

    // Don't do anything if we did not find Roon UI
    if pid == Pid::from(0) {
        println!("No Roon running, not bothering");
        return;
    }
    match &state.subprocess {
        Some(_) => {
            // println!("cpulimit already running, doing nothing");
        }
        _ => {
            println!("Calling cpulimit on PID {:?}", pid);
            state.subprocess = Some(
                Command::new("cpulimit")
                    .arg("-p")
                    .arg(pid.to_string())
                    .arg("-l")
                    .arg("10")
                    .spawn()
                    .expect("cpulimit command failed to start"),
            );
        }
    }
}

fn on_window(event: Box<WindowData>, state: &mut State) {
    // println!("{:?}", event);

    if event.change == event::WindowChange::Focus {
        let window_prop = match event.container.window_properties {
            Some(w) => w,
            _ => return,
        };
        //println!("{:?}", window_prop);

        let class = window_prop.class.unwrap_or("".to_string());
        if class == "roon.exe" {
            on_roon(state);
        } else {
            not_on_roon(state);
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut state = State {
        subprocess: None,
        system: System::new_with_specifics(
            RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
        ),
    };

    let mut i3 = I3::connect().await?;
    let resp = i3.subscribe([Subscribe::Window]).await?;
    println!("{:#?}", resp);

    let mut listener = i3.listen();
    while let Some(event) = listener.next().await {
        match event? {
            Event::Window(ev) => on_window(ev, &mut state),
            // ignore all other events
            _ => continue,
        }
    }
    Ok(())
}
