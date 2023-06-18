use std::io;
use tokio_i3ipc::{
    event::{self, Event, Subscribe, WindowData},
    I3,
};
use tokio_stream::StreamExt;

use sysinfo::{ProcessExt, ProcessRefreshKind, RefreshKind, System, SystemExt};

use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::process::{Child, Command};

// Embed my world in one ugly struct
struct State {
    subprocess: Option<Child>,
    system: System,
}

fn on_roon(state: &mut State) {
    // We found out we were focused on Roon, lift the CPU limitation
    println!("Focused on Roon");
    if let Some(p) = &mut state.subprocess {
        println!("Terminating cpulimit subprocess");
        signal::kill(Pid::from_raw(p.id() as i32), Signal::SIGTERM).unwrap();
        let _ = p.wait();
        println!("Did kill and wait");
        state.subprocess = None;
    } else {
        println!("No cpulimit subprocess to kill, which is unexpected...");
    }
}

fn not_on_roon(state: &mut State) {
    // refresh the list of processes
    state
        .system
        .refresh_processes_specifics(ProcessRefreshKind::new());

    // Assume at most only one process matches "Roon.exe"
    if let Some(process) = state.system.processes_by_exact_name("Roon.exe").next() {
        //println!("Found roon at PID {:?}", process);
        match &state.subprocess {
            None => {
                println!("Calling cpulimit on PID {:?}", process.pid());

                state.subprocess = Some(
                    Command::new("cpulimit")
                        .arg("-p")
                        .arg(process.pid().to_string())
                        .arg("-l")
                        .arg("10")
                        .spawn()
                        .expect("cpulimit command failed to start"),
                );
            }
            Some(_) => (),
        }
    } else {
        println!("Not found roon");
    }
}

fn on_window(event: Box<WindowData>, state: &mut State) {
    if event.change != event::WindowChange::Focus {
        return;
    }
    let Some(window_prop) = event.container.window_properties else { return };
    //println!("{:?}", window_prop);

    if window_prop.class == Some("roon.exe".to_string()) {
        on_roon(state);
    } else {
        not_on_roon(state);
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
    i3.subscribe([Subscribe::Window]).await?;

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
