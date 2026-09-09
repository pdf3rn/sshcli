use std::sync::mpsc;

use i_slint_backend_testing::init_no_event_loop;
use slint::{ComponentHandle, Model, SharedString};
use sshcli_terminal_ui::{TerminalCommand, TerminalController, TerminalEvent, TerminalSurface};

#[test]
fn terminal_surface_delivers_events_and_refreshes_large_models_headlessly() {
    // The no-event-loop backend is deterministic and does not require a display.
    // Keep all callback and model assertions in one test because Slint's test
    // backend is process-global for the duration of a test binary.
    init_no_event_loop();

    let surface = TerminalSurface::new().expect("create terminal surface");
    let inputs = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
    let resizes = std::rc::Rc::new(std::cell::RefCell::new(Vec::<(i32, i32)>::new()));
    let closes = std::rc::Rc::new(std::cell::RefCell::new(0_u32));

    {
        let inputs = inputs.clone();
        surface.on_input(move |input| inputs.borrow_mut().push(input.to_string()));
    }
    {
        let resizes = resizes.clone();
        surface.on_resize(move |columns, rows| resizes.borrow_mut().push((columns, rows)));
    }
    {
        let closes = closes.clone();
        surface.on_close(move || *closes.borrow_mut() += 1);
    }

    surface.show().expect("show headless surface");
    // The surface's forward-focus target receives window key events.
    surface
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: SharedString::from("a"),
        });
    surface
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased {
            text: SharedString::from("a"),
        });
    surface
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed {
            text: SharedString::from("\u{1b}[A"),
        });
    assert_eq!(&*inputs.borrow(), &["a", "\u{1b}[A"]);

    // TerminalSurface is an exported component rather than a Window, so the
    // no-event-loop backend cannot resize its host geometry. Invoke the typed
    // callback directly to verify delivery at this boundary; geometry-driven
    // negotiation remains covered by the real window/runtime path.
    surface.invoke_resize(100, 33);
    assert!(resizes
        .borrow()
        .iter()
        .any(|&(columns, rows)| columns == 100 && rows == 33));

    surface.invoke_close();
    assert_eq!(*closes.borrow(), 1);

    let (event_tx, event_rx) = mpsc::channel();
    let (command_tx, _command_rx) = mpsc::channel::<TerminalCommand>();
    let mut controller = TerminalController::new(80, 24, event_rx, command_tx);
    event_tx
        .send(TerminalEvent::Output(vec![b'x'; 80 * 24]))
        .expect("send large output");
    assert!(controller.pump());
    controller.refresh_surface(&surface);
    assert_eq!(surface.get_status(), "Ready");
    assert_eq!(surface.get_cells().row_count(), 80 * 24);
    assert_eq!(
        surface.get_cells().row_data(80 * 24 - 1).unwrap().character,
        "x"
    );

    event_tx
        .send(TerminalEvent::Error("pty failed".into()))
        .expect("send error event");
    assert!(controller.pump());
    controller.refresh_surface(&surface);
    assert_eq!(surface.get_status(), "Error: pty failed");
}
