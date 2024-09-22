use std::sync::mpsc;
use std::{error::Error, time::Duration};
use std::{io, thread};

use crossterm::event::{self, Event, KeyCode};
use crossterm::{
    cursor::{Hide, Show},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use invaders::frame::new_frame;
use invaders::{frame, render};
use rusty_audio::Audio;

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = Audio::new();
    audio.add("explode", "assets/sounds/explode.wav");
    audio.add("lose", "assets/sounds/lose.wav");
    audio.add("move", "assets/sounds/move.wav");
    audio.add("pew", "assets/sounds/pew.wav");
    audio.add("startup", "assets/sounds/startup.wav");
    audio.add("win", "assets/sounds/win.wav");
    audio.play("startup");

    // terminal
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?; // hide cursor

    // render loop in a separate thread
    // in a real project, use crossbeam rather than built-in mpsc channels
    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || {
        let mut last_frame = frame::new_frame();
        let mut stdout = io::stdout();
        // set up screen and force render everything
        render::render(&mut stdout, &last_frame, &last_frame, true);
        // in this loop, we receive frames and then render them
        loop {
            let curr_frame = match render_rx.recv() {
                Ok(frame) => frame,
                Err(_) => break,
            };
            render::render(&mut stdout, &last_frame, &curr_frame, false);
            last_frame = curr_frame;
        }
    });

    // game loop
    'gameloop: loop {
        // per-frame init
        let curr_frame = new_frame();

        // input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameloop;
                    }
                    _ => (),
                }
            }
        }

        /*
            draw and render
            this will be received and handled by the rendering loop
            will probably fail first few times, because the child thread won't have been set up -> ignore error
        */
        let _ = render_tx.send(curr_frame);
        // tiny sleep to stop frames constantly being rendered
        thread::sleep(Duration::from_millis(1));
    }

    // cleanup
    drop(render_tx); // not needed in newer versions of rust
                     // after this point, the render loop will exit -> can now join on render_handle
    render_handle.join().unwrap();
    audio.wait(); // audio plays on a separate thread
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    Ok(())
}
