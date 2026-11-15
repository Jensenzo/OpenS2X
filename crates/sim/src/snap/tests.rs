//! A world written to a snapshot and read back is the same world, and
//! stepping it gives the same game as stepping the one saved.

use super::{Reader, Shape, Snap, SnapError, Writer};
use crate::combat_tests::{attack, standard};
use crate::World;

/// Save at every tick of `ticks`; each loaded world equals the saved one,
/// writes the same bytes again, and `ahead` ticks later still equals the
/// game that was never saved.
fn round_trips(mut world: World, ticks: u64, ahead: u64) {
    let first = world.tick();
    let mut hashes = Vec::new();
    let mut loaded_games = Vec::new();
    for _ in 0..ticks {
        let bytes = world.snapshot();
        let loaded = World::from_snapshot(&bytes).expect("snapshot reads back");
        assert_eq!(loaded, world, "tick {}", world.tick());
        assert_eq!(loaded.snapshot(), bytes, "tick {}", world.tick());
        loaded_games.push(loaded);
        hashes.push(world.state_hash());
        world.step();
    }
    for _ in 0..ahead {
        hashes.push(world.state_hash());
        world.step();
    }
    for mut game in loaded_games {
        for _ in 0..ahead {
            game.step();
            let at = (game.tick() - first) as usize;
            assert_eq!(game.state_hash(), hashes[at], "tick {}", game.tick());
        }
    }
}

// spec: none (our own snapshot format, docs/snapshot.md: an attack with duels)
#[test]
fn a_battle_saved_at_every_tick_continues_the_same() {
    let mut world = standard(4, 2);
    attack(&mut world, 3, false);
    round_trips(world, 120, 40);
}

// spec: none (our own snapshot format, docs/snapshot.md: a new game's economy)
#[test]
fn a_new_game_saved_at_every_tick_continues_the_same() {
    let world = crate::requests::tests::fresh_game();
    round_trips(world, 150, 30);
}

// spec: none (our own snapshot format, docs/snapshot.md)
#[test]
fn a_broken_snapshot_is_refused() {
    let world = standard(1, 1);
    let bytes = world.snapshot();
    assert!(World::from_snapshot(&bytes[..bytes.len() / 2]).is_err());
    assert!(World::from_snapshot(b"not a snapshot").is_err());
    let mut longer = bytes.clone();
    longer.push(0);
    assert!(World::from_snapshot(&longer).is_err());
    let mut other_version = bytes;
    other_version[8] = 99;
    let Err(SnapError(text)) = World::from_snapshot(&other_version) else {
        panic!("a different version reads");
    };
    assert!(text.contains("version 99"), "{text}");
}

// spec: none (our own snapshot format, docs/snapshot.md: a save of an earlier format loads; older than the first or newer is refused)
#[test]
fn only_formats_from_the_oldest_to_this_one_are_read() {
    let world = standard(1, 1);
    let bytes = world.snapshot();
    assert_eq!(World::from_snapshot(&bytes).as_ref(), Ok(&world));
    let with_format = |format: u32| {
        let mut w = Writer::new();
        w.raw(super::MAGIC);
        w.u64(u64::from(format));
        w.raw(&bytes[8 + version_bytes()..]);
        w.bytes
    };
    for format in [0, super::OLDEST - 1, super::VERSION + 1, u32::MAX] {
        let Err(SnapError(text)) = World::from_snapshot(&with_format(format)) else {
            panic!("format {format} reads");
        };
        assert!(text.contains(&format!("version {format},")), "{text}");
        assert!(
            text.contains(&format!("{} to {}", super::OLDEST, super::VERSION)),
            "{text}"
        );
    }
}

fn version_bytes() -> usize {
    let mut w = Writer::new();
    w.u64(u64::from(super::VERSION));
    w.bytes.len()
}

/// A type as it was saved in format 65.
#[derive(Debug, PartialEq)]
struct SavedThen {
    a: u32,
    b: String,
}
crate::snap_struct!(SavedThen { a, b });

/// The same type after two later rules: a field with a default, and one
/// made from what the older bytes hold at its place.
#[derive(Debug, PartialEq)]
struct ReadNow {
    a: u32,
    b: String,
    hidden: bool,
    tally: Vec<u32>,
}
crate::snap_struct!(ReadNow {
    a,
    b,
    hidden @ 66 = true,
    tally @ 67 => |_: &mut Reader| Ok::<_, SnapError>(vec![1, 2])
});

#[derive(Debug, PartialEq)]
enum StepThen {
    Wait,
    Walk { to: u32 },
    Fight(u32),
}
crate::snap_enum!(StepThen { Wait, Walk { to }, Fight(f0) });

#[derive(Debug, PartialEq)]
enum StepNow {
    Wait,
    Walk {
        to: u32,
        back: bool,
    },
    Fight(u32),
    /// Added: at the end of the macro's list.
    Rest,
}
crate::snap_enum!(StepNow { Wait, Walk { to, back @ 66 = false }, Fight(f0), Rest });

fn bytes_of<T: Snap>(value: &T) -> Vec<u8> {
    let mut w = Writer::new();
    value.put(&mut w);
    w.bytes
}

// spec: none (our own snapshot format, docs/snapshot.md: a field added later takes its default in an older save)
#[test]
fn a_field_added_later_takes_its_default_in_older_bytes() {
    let then = bytes_of(&SavedThen {
        a: 5,
        b: "x".into(),
    });
    let mut r = Reader::of_format(&then, 65);
    assert_eq!(
        r.get::<ReadNow>(),
        Ok(ReadNow {
            a: 5,
            b: "x".into(),
            hidden: true,
            tally: vec![1, 2],
        })
    );
    assert!(r.at_end());
    // Bytes of the format that has it read it, and what it writes reads back.
    let now = ReadNow {
        a: 6,
        b: "y".into(),
        hidden: false,
        tally: vec![9],
    };
    let bytes = bytes_of(&now);
    let mut r = Reader::of_format(&bytes, 67);
    assert_eq!(r.get::<ReadNow>().as_ref(), Ok(&now));
    assert!(r.at_end());
    // Between the two: the first field read, the second from its default.
    let mut w = Writer::new();
    SavedThen {
        a: 7,
        b: "z".into(),
    }
    .put(&mut w);
    w.put(&false);
    let mut r = Reader::of_format(&w.bytes, 66);
    let read: ReadNow = r.get().expect("format 66");
    assert_eq!((read.hidden, read.tally), (false, vec![1, 2]));
    assert!(r.at_end());
}

// spec: none (our own snapshot format, docs/snapshot.md: a variant added at the end leaves the older variants' numbers)
#[test]
fn a_variant_added_at_the_end_keeps_older_bytes_readable() {
    for (then, now) in [
        (StepThen::Wait, StepNow::Wait),
        (
            StepThen::Walk { to: 3 },
            StepNow::Walk { to: 3, back: false },
        ),
        (StepThen::Fight(4), StepNow::Fight(4)),
    ] {
        let bytes = bytes_of(&then);
        let mut r = Reader::of_format(&bytes, 65);
        assert_eq!(r.get::<StepNow>().as_ref(), Ok(&now));
        assert!(r.at_end());
    }
    let bytes = bytes_of(&StepNow::Rest);
    assert_eq!(Reader::of_format(&bytes, 66).get(), Ok(StepNow::Rest));
    // A variant the reader does not know is refused, not guessed.
    assert!(Reader::of_format(&bytes, 66).get::<StepThen>().is_err());
}

fn shape_of<T: Snap>() -> String {
    let mut s = Shape::new();
    s.add::<T>();
    s.text()
}

// spec: none (our own snapshot format, docs/snapshot.md: the format written out names every field, variant and later format)
#[test]
fn the_format_written_out_names_every_field_and_variant() {
    let text = shape_of::<ReadNow>();
    assert!(text.contains("    a: u32\n"), "{text}");
    assert!(text.contains("    b: string\n"), "{text}");
    assert!(text.contains("    hidden @66: bool\n"), "{text}");
    assert!(text.contains("    tally @67=>: seq<u32>\n"), "{text}");
    let text = shape_of::<StepNow>();
    assert!(text.contains("    0 Wait\n"), "{text}");
    assert!(
        text.contains("    1 Walk { to: u32, back @66: bool }\n"),
        "{text}"
    );
    assert!(text.contains("    2 Fight { f0: u32 }\n"), "{text}");
    assert!(text.contains("    3 Rest\n"), "{text}");
    // A new field or variant changes the text: the world's has them all.
    assert_ne!(shape_of::<StepThen>(), shape_of::<StepNow>());
    let world = shape_of::<World>();
    assert!(world.contains("s2_sim::World\n    tick: u64\n"), "{world}");
    assert!(world.contains("s2_sim::rng::Rng\n    state: "), "{world}");
}

// spec: none (our own snapshot format, docs/snapshot.md: the encoding)
#[test]
fn values_round_trip() {
    fn again<T: Snap + PartialEq + std::fmt::Debug>(value: T) {
        let mut w = super::Writer::new();
        value.put(&mut w);
        let mut r = super::Reader::new(&w.bytes);
        assert_eq!(T::get(&mut r).unwrap(), value);
        assert!(r.at_end());
    }
    again(0u64);
    again(u64::MAX);
    again(i64::MIN);
    again(-1i32);
    again(i32::MAX);
    again(Some((3u8, "text".to_string())));
    again(vec![Some(1usize), None]);
    again([1u32, 2, 3]);
    let ok: Result<(), u32> = Err(5);
    again(ok);
}
