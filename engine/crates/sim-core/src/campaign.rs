//! Original "Together, onward" campaign: three cooperative foundations, twelve
//! mechanism lessons, five remixes, and four large 2/3/4-player expeditions.
//! Units are 1/1024 tile.
use crate::*;

pub const LEVEL_COUNT: u8 = 28;

fn r(x: i32, y: i32, w: i32, h: i32) -> Rect {
    Rect {
        x: x * UNIT,
        y: y * UNIT,
        w: w * UNIT,
        h: h * UNIT,
    }
}

fn add(l: &mut Level, kind: MechanicKind, rect: Rect, travel: i32, period: u32) {
    l.mechanics.push(Mechanic {
        id: l.mechanics.len() as u32 + 1,
        kind,
        rect,
        travel,
        period,
    });
}

fn hazard(l: &mut Level, x: i32, width: i32) {
    add(
        l,
        MechanicKind::Hazard,
        Rect {
            y: 12 * UNIT - UNIT / 4,
            h: UNIT / 4,
            ..r(x, 0, width, 1)
        },
        0,
        0,
    );
}

// A normal-speed jump must clear the strip plus the player's width. Wider
// hazards below ferries/bridges deliberately require those traversal tools.
fn hurdle(l: &mut Level, x: i32) {
    hazard(l, x, 1);
    l.mechanics.last_mut().unwrap().rect.w = 5 * UNIT / 4;
}

fn plate_gate(l: &mut Level, x: i32) {
    add(
        l,
        MechanicKind::Plate,
        Rect {
            y: 12 * UNIT - UNIT / 8,
            h: UNIT / 8,
            ..r(x, 0, 2, 1)
        },
        0,
        0,
    );
    add(l, MechanicKind::PressureGate, r(x + 4, 8, 1, 4), 0, 240);
}

fn checkpoint(l: &mut Level, x: i32) {
    add(l, MechanicKind::Checkpoint, r(x, 10, 4, 2), 0, 0);
}

/// 1-based catalog. Invalid indices never silently select another level.
pub fn level(number: u8) -> Option<Level> {
    use MechanicKind::*;
    if !(1..=LEVEL_COUNT).contains(&number) {
        return None;
    }
    if number > 24 {
        return Some(platform_course(number));
    }
    if number > 20 {
        return Some(expedition(number));
    }
    let (title, hint) = match number {
        1 => (
            "A little lift",
            "A friend is a step. Release jump before jumping from their shoulders.",
        ),
        2 => (
            "One key, two paths",
            "The high key opens the wall for everyone. Meet again beyond it.",
        ),
        3 => (
            "Wait for me",
            "The exit needs the whole team. Use the terrace to reach the key together.",
        ),
        4 => (
            "Hold the welcome",
            "Stand on the brass pad. The door stays open four seconds after release.",
        ),
        5 => (
            "A breath between beats",
            "The striped door opens for half its cycle. Wait together, then cross.",
        ),
        6 => (
            "Through the underside",
            "Thin ledges catch falling feet, but let you jump through from below.",
        ),
        7 => (
            "The patient ferry",
            "Ride the horizontal shuttle above the red strip; it waits rather than crushes.",
        ),
        8 => (
            "Against the grain",
            "The belt pushes left. Walk or jump against it.",
        ),
        9 => (
            "Room to brake",
            "Ice preserves momentum. Countersteer before the small red strip.",
        ),
        10 => (
            "Borrowed bounce",
            "The green pad launches you to the high key without pressing jump.",
        ),
        11 => (
            "Mind the seams",
            "Red strips reset the whole team. Cross each gap with a fresh jump.",
        ),
        12 => (
            "A place to return",
            "Touch the flag together before the long crossing. The key survives a fall.",
        ),
        13 => (
            "Not for standing",
            "The fragile shelf disappears after contact, then rebuilds. Keep moving.",
        ),
        14 => (
            "Lighter together",
            "Inside the blue field a jump rises much higher. Reach the floating key.",
        ),
        15 => (
            "Lean into it",
            "The wind pushes right even in the air. Leave room before the red strip.",
        ),
        16 => (
            "Relay at dusk",
            "Open the first door, then time the second. The flag saves your progress.",
        ),
        17 => (
            "Cold machinery",
            "Countersteer on ice, then ride the ferry over the wide danger strip.",
        ),
        18 => (
            "Updraft workshop",
            "Bounce into light gravity for the key. Thin shelves catch your return.",
        ),
        19 => (
            "A bridge on loan",
            "Use the flag, cross the temporary bridge, then release the pressure door.",
        ),
        _ => (
            "The long way home",
            "Combine timing, momentum and a shared lift. Regroup at both flags.",
        ),
    };
    let mut l = Level {
        number,
        title: title.into(),
        hint: hint.into(),
        ..Level::default()
    };
    if number == 1 {
        return Some(l);
    }
    let width = if number <= 3 {
        24 + i32::from(number)
    } else {
        28 + i32::from(number / 3) * 2
    };
    l.width = width * UNIT;
    l.solids = vec![r(0, 12, width, 2)];
    l.exit = r(width - 3, 10, 2, 2);
    l.gate = r(width - 6, 9, 1, 3);
    l.key = Rect {
        w: UNIT / 2,
        h: UNIT / 2,
        ..r(width - 9, 11, 1, 1)
    };
    match number {
        2 => {
            l.gate = r(11, 9, 3, 3);
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(12, 8, 1, 1)
            };
        }
        3 => {
            l.solids.push(r(7, 11, 2, 1));
            l.gate = r(11, 9, 2, 3);
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(11, 8, 1, 1)
            };
        }
        4 => plate_gate(&mut l, 8),
        5 => add(&mut l, TimedGate, r(12, 7, 1, 5), 0, 240),
        6 => {
            add(
                &mut l,
                OneWay,
                Rect {
                    h: UNIT / 4,
                    ..r(8, 10, 3, 1)
                },
                0,
                0,
            );
            add(
                &mut l,
                OneWay,
                Rect {
                    h: UNIT / 4,
                    ..r(12, 8, 3, 1)
                },
                0,
                0,
            );
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(13, 7, 1, 1)
            };
        }
        7 => {
            hazard(&mut l, 11, 4);
            add(
                &mut l,
                MovingPlatform,
                Rect {
                    h: UNIT / 2,
                    ..r(9, 10, 4, 1)
                },
                3 * UNIT,
                300,
            );
        }
        8 => {
            add(&mut l, Conveyor, r(8, 12, 9, 1), -64, 0);
            l.solids.push(r(18, 11, 1, 1));
        }
        9 => {
            add(&mut l, Ice, r(8, 12, 9, 1), 0, 0);
            hazard(&mut l, 18, 1);
        }
        10 => {
            add(
                &mut l,
                Spring,
                Rect {
                    y: 12 * UNIT - UNIT / 8,
                    h: UNIT / 8,
                    ..r(10, 0, 2, 1)
                },
                0,
                0,
            );
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(11, 7, 1, 1)
            };
        }
        11 => {
            hazard(&mut l, 10, 1);
            hurdle(&mut l, 16);
            hazard(&mut l, 23, 1);
        }
        12 => {
            checkpoint(&mut l, 7);
            hurdle(&mut l, 13);
            hurdle(&mut l, 19);
        }
        13 => {
            hazard(&mut l, 12, 4);
            add(
                &mut l,
                Crumble,
                Rect {
                    h: UNIT / 2,
                    ..r(10, 10, 8, 1)
                },
                0,
                70,
            );
        }
        14 => {
            add(&mut l, LowGravity, r(8, 3, 10, 9), 0, 0);
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(13, 5, 1, 1)
            };
        }
        15 => {
            add(&mut l, Wind, r(8, 6, 13, 6), 48, 0);
            hurdle(&mut l, 22);
        }
        16 => {
            plate_gate(&mut l, 8);
            checkpoint(&mut l, 14);
            add(&mut l, TimedGate, r(21, 7, 1, 5), 0, 180);
            hazard(&mut l, 25, 1);
        }
        17 => {
            add(&mut l, Ice, r(7, 12, 5, 1), 0, 0);
            add(&mut l, Conveyor, r(13, 12, 3, 1), -48, 0);
            checkpoint(&mut l, 12);
            hazard(&mut l, 19, 4);
            add(
                &mut l,
                MovingPlatform,
                Rect {
                    h: UNIT / 2,
                    ..r(17, 10, 4, 1)
                },
                3 * UNIT,
                240,
            );
        }
        18 => {
            add(
                &mut l,
                Spring,
                Rect {
                    y: 12 * UNIT - UNIT / 8,
                    h: UNIT / 8,
                    ..r(9, 0, 2, 1)
                },
                0,
                0,
            );
            add(&mut l, LowGravity, r(8, 0, 10, 12), 0, 0);
            add(
                &mut l,
                OneWay,
                Rect {
                    h: UNIT / 4,
                    ..r(16, 8, 4, 1)
                },
                0,
                0,
            );
            add(&mut l, Wind, r(11, 2, 9, 8), 24, 0);
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(14, 3, 1, 1)
            };
            hurdle(&mut l, 23);
        }
        19 => {
            checkpoint(&mut l, 7);
            hazard(&mut l, 13, 5);
            add(
                &mut l,
                Crumble,
                Rect {
                    h: UNIT / 2,
                    ..r(11, 10, 9, 1)
                },
                0,
                90,
            );
            plate_gate(&mut l, 21);
            add(&mut l, TimedGate, r(29, 7, 1, 5), 0, 180);
        }
        20 => {
            plate_gate(&mut l, 7);
            checkpoint(&mut l, 13);
            add(&mut l, Ice, r(17, 12, 4, 1), 0, 0);
            hazard(&mut l, 22, 3);
            add(
                &mut l,
                MovingPlatform,
                Rect {
                    h: UNIT / 2,
                    ..r(20, 10, 3, 1)
                },
                3 * UNIT,
                240,
            );
            checkpoint(&mut l, 26);
            add(&mut l, TimedGate, r(30, 7, 1, 5), 0, 150);
            // Last key reprises the opening shared lift rather than another floor pickup.
            l.gate = r(34, 9, 2, 3);
            l.key = Rect {
                w: UNIT / 2,
                h: UNIT / 2,
                ..r(35, 8, 1, 1)
            };
        }
        _ => unreachable!(),
    }
    Some(l)
}

fn expedition(number: u8) -> Level {
    use MechanicKind::*;
    let index = i32::from(number - 21);
    let width = 64 + index * 8;
    let height = 32 + index * 4;
    let floor = height - 2;
    let required = if number <= 22 { 2 } else { number - 20 };
    let mut l = Level {
        number, required_players: required, spawn_y: floor * UNIT,
        width: width * UNIT, height: height * UNIT,
        title: match number { 21 => "The shared rope", 22 => "Quarry crossing", 23 => "Three on the line", _ => "Four to the summit" }.into(),
        hint: format!("{required} players needed. Line up on the pads to tension the rope; hold Jump together to climb to the key. Regroup below, watch the falling-rock warnings, then cross the long valley."),
        solids: vec![r(0, floor, width, 2)],
        // The key wall shares the pressure-gate shaft: nobody can activate the
        // far-side checkpoint before fetching the high key, avoiding a softlock.
        gate: r(20, 2, 1, floor - 2),
        key: Rect { w: UNIT / 2, h: UNIT / 2, ..r(9 + 2 * i32::from(required - 1), 6, 1, 1) },
        exit: r(width - 3, floor - 2, 2, 2),
        ..Level::default()
    };
    add(&mut l, Rope, r(8, 2, 9, floor - 2), 0, 0);
    for i in 0..required {
        add(
            &mut l,
            Plate,
            Rect {
                y: floor * UNIT - UNIT / 8,
                h: UNIT / 8,
                ..r(9 + i32::from(i) * 2, 0, 1, 1)
            },
            0,
            0,
        );
    }
    add(&mut l, PressureGate, r(20, 2, 1, floor - 2), 0, 3600);
    add(&mut l, Checkpoint, r(23, floor - 2, 4, 2), 0, 0);
    for x in [32, width - 19] {
        add(&mut l, FallingRock, r(x, floor - 9, 1, 1), 8 * UNIT, 240);
    }
    l
}

fn platform_course(number: u8) -> Level {
    let index = i32::from(number - 25);
    let width = 128 + index * 32;
    let height = [56, 64, 80, 96][index as usize];
    let floor = height - 10; // Enough void clearance for a tether rescue.
    let columns = (width - 20) / 7;
    let mut route = vec![r(0, floor, 12, 2)];
    for step in 1..=columns {
        route.push(Rect {
            h: UNIT / 2,
            ..r(13 + (step - 1) * 7, floor - step, 6, 1)
        });
    }
    if number >= 26 {
        let end = route.last().unwrap().x / UNIT;
        for step in 1..columns {
            route.push(Rect {
                h: UNIT / 2,
                ..r(end - step * 7, floor - columns - step, 6, 1)
            });
        }
    }
    let last = *route.last().unwrap();
    let middle = route[route.len() / 2];
    let mut l = Level {
        number, width: width * UNIT, height: height * UNIT, spawn_y: floor * UNIT,
        required_players: if number <= 26 { 2 } else { number - 24 },
        tether_length: if number == 26 { 0 } else { 6 * UNIT },
        title: match number { 25 => "Tied at the ridge", 26 => "The long switchback", 27 => "Three across the sky", _ => "The high road home" }.into(),
        hint: if number == 26 {
            "A long rising platform course turns back above the first route. Jump the gaps, regroup at flags, and bring the key to the upper exit."
        } else {
            "You are tied together. Keep slack for jumps, wait on safe ledges, and let an anchored teammate catch a fall. Climb the rising platforms together; flags regroup the whole team."
        }.into(),
        solids: route[..(columns as usize + 1)].to_vec(),
        key: Rect { x: middle.x + 2 * UNIT, y: middle.y - UNIT, w: UNIT / 2, h: UNIT / 2 },
        gate: Rect { x: last.x + 4 * UNIT, y: last.y - 3 * UNIT, w: UNIT, h: 3 * UNIT },
        exit: Rect { x: last.x + 4 * UNIT, y: last.y - 2 * UNIT, w: 2 * UNIT, h: 2 * UNIT },
        ..Level::default()
    };
    // Upper switchbacks are one-way ledges: they must not form low ceilings
    // that prevent jumping along the lower outbound route.
    for platform in route.iter().skip(columns as usize + 1) {
        add(&mut l, MechanicKind::OneWay, *platform, 0, 0);
    }
    for platform in route.iter().skip(1).step_by(5) {
        add(
            &mut l,
            MechanicKind::Checkpoint,
            Rect {
                x: platform.x - UNIT,
                y: platform.y - 2 * UNIT,
                w: 6 * UNIT,
                h: 2 * UNIT,
            },
            0,
            0,
        );
    }
    l
}

#[cfg(test)]
mod tests;
