use arcana::{Event, VersionedEvent};

#[derive(VersionedEvent)]
#[event(name = "chat", version = 1)]
struct ChatEvent;

#[derive(VersionedEvent)]
#[event(name = "file", version = 1)]
struct FileEvent;

#[derive(Event)]
enum AnyEvent {
    Chat(ChatEvent),
    File { event: FileEvent },
}

fn main() {
    let ev = AnyEvent::Chat(ChatEvent);
    assert_eq!(ev.name(), "chat");

    let ev = AnyEvent::File { event: FileEvent };
    assert_eq!(ev.name(), "file");
}