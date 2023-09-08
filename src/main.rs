use std::{cell::RefCell, str::FromStr};

use i3ipc::{
    event::BindingEventInfo,
    reply::{Node, NodeLayout},
    I3Connection, I3EventListener, Subscription,
};

#[derive(PartialEq)]
enum I3Split {
    Vertical,
    Horizontal,
    Tabbed,
    Stacked,
    Toggle,
}

struct I3SplitParseError;

impl FromStr for I3Split {
    type Err = I3SplitParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "v" | "vertical" => Ok(I3Split::Vertical),
            "h" | "horizontal" => Ok(I3Split::Horizontal),
            "tabbed" => Ok(I3Split::Tabbed),
            "stacked" | "stacking" => Ok(I3Split::Stacked),
            "t" | "toggle" => Ok(I3Split::Toggle),
            _ => Err(I3SplitParseError),
        }
    }
}

thread_local! {
    static PREVIOUS_SPLIT: RefCell<I3Split> = RefCell::new(I3Split::Horizontal);
}

fn main() {
    let mut i3 = I3Connection::connect().expect("Problem connecting to i3");
    let mut i3_events = I3EventListener::connect().expect("Problem connecting to i3");

    i3_events
        .subscribe(&[Subscription::Window, Subscription::Binding])
        .expect("Problem subscribing to events");

    i3_events.listen().for_each(|event| {
        let Ok(event) = event else {
            eprintln!("Error: {event:?}");
            return;
        };

        match event {
            i3ipc::event::Event::WindowEvent(_) => set_layout(&mut i3),
            i3ipc::event::Event::BindingEvent(e) => handle_keybind(&mut i3, e),
            _ => unreachable!(),
        };
    })
}

fn set_layout(i3: &mut I3Connection) -> Option<()> {
    fn find_focused_parent(node: &Node) -> Option<&Node> {
        if node.nodes.iter().any(|n| n.focused) {
            Some(node)
        } else {
            node.nodes.iter().find_map(find_focused_parent)
        }
    }

    let tree = i3.get_tree().ok()?;
    let parent = find_focused_parent(&tree);
    match parent {
        Some(parent) => {
            if matches!(parent.layout, NodeLayout::Tabbed | NodeLayout::Stacked) {
                print_status(match parent.layout {
                    NodeLayout::Tabbed => I3Split::Tabbed,
                    NodeLayout::Stacked => I3Split::Stacked,
                    _ => unreachable!(),
                })
            } else if parent.rect.2 > parent.rect.3 {
                // rect: (x, y, width, height)
                i3.run_command("split horizontal").ok()?;
                print_status(I3Split::Horizontal)
            } else {
                i3.run_command("split vertical").ok()?;
                print_status(I3Split::Vertical)
            }
        }
        None => println!(),
    }

    Some(())
}

fn handle_keybind(i3: &mut I3Connection, e: BindingEventInfo) -> Option<()> {
    let mut binding = e.binding.command.split(' ');
    match binding.next()? {
        "split" => print_status(binding.next()?.parse().ok()?),
        "move" | "focus" | "workspace" => set_layout(i3)?,
        "layout" => {
            let command = binding.next()?;
            let split = if command.starts_with("split") {
                // layout splith, splitv
                command.chars().last()?.to_string()
            } else {
                command.to_owned()
            };

            print_status(split.parse().ok()?)
        }
        _ => {}
    }

    Some(())
}

fn print_status(split: I3Split) {
    match split {
        I3Split::Tabbed => println!("t"),
        I3Split::Stacked => println!("s"),
        I3Split::Vertical => PREVIOUS_SPLIT.with(|prev| {
            *prev.borrow_mut() = I3Split::Vertical;
            println!(" ↓")
        }),
        I3Split::Horizontal => PREVIOUS_SPLIT.with(|prev| {
            *prev.borrow_mut() = I3Split::Horizontal;
            println!("→")
        }),
        I3Split::Toggle => PREVIOUS_SPLIT.with(|prev| {
            if *prev.borrow() == I3Split::Vertical {
                print_status(I3Split::Horizontal)
            } else if *prev.borrow() == I3Split::Horizontal {
                print_status(I3Split::Vertical)
            }
        }),
    }
}
