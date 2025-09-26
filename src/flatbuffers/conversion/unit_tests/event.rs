use crate::{
    flatbuffers::conversion::{FromFlatbuffers, ToFlatbuffers},
    tasks::{config::StreamSource, event::TaskEvent},
};

#[test]
fn event_started_roundtrip() {
    let event = TaskEvent::Started {
        task_name: "test_task".to_string(),
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}
#[test]
fn event_output_roundtrip() {
    let event = TaskEvent::Output {
        task_name: "test_task".to_string(),
        line: "Hello, World!".to_string(),
        src: StreamSource::Stdout,
    };

    let mut builder = flatbuffers::FlatBufferBuilder::new();
    let fb_event = event.to_flatbuffers(&mut builder);
    builder.finish(fb_event, None);

    let bytes = builder.finished_data();
    assert!(!bytes.is_empty());

    // Roundtrip: deserialize and compare
    let fb_event =
        flatbuffers::root::<crate::flatbuffers::tcrm_task_generated::tcrm::task::TaskEvent>(bytes)
            .unwrap();
    let roundtripped = TaskEvent::from_flatbuffers(fb_event).unwrap();
    assert_eq!(event, roundtripped);
}
