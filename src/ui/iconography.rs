//! The shared 24x24 icon language for Master Thief's responsive chrome.
//!
//! Every mark is drawn on the same grid with the same rounded-corner intent and
//! a single readable stroke. Labels remain beside touch targets; these glyphs
//! add recognition and state, they never become the only way to act.

use macroquad::prelude::*;

const GRID: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Monogram,
    Crew,
    Board,
    Outfitter,
    LastJob,
    Records,
    Save,
    Load,
    Delete,
    Settings,
    Sound,
    Music,
    Fullscreen,
    Help,
    Close,
    Back,
    Warning,
    Info,
    Success,
    Danger,
    Search,
    Filter,
    Sort,
    Add,
    Remove,
    Lock,
    Unlock,
    Notification,
    CaseFile,
    Target,
    Payout,
    Reputation,
    Notoriety,
    Heat,
    Safehouse,
    Payroll,
    Doctor,
    Fence,
    Bail,
    Retire,
    Door,
    Dice,
    Advance,
    NewCampaign,
    Quieter,
    Louder,
    Hints,
    Change,
    Mute,
    Unmute,
}

impl Icon {
    pub const ALL: [Icon; 50] = [
        Icon::Monogram,
        Icon::Crew,
        Icon::Board,
        Icon::Outfitter,
        Icon::LastJob,
        Icon::Records,
        Icon::Save,
        Icon::Load,
        Icon::Delete,
        Icon::Settings,
        Icon::Sound,
        Icon::Music,
        Icon::Fullscreen,
        Icon::Help,
        Icon::Close,
        Icon::Back,
        Icon::Warning,
        Icon::Info,
        Icon::Success,
        Icon::Danger,
        Icon::Search,
        Icon::Filter,
        Icon::Sort,
        Icon::Add,
        Icon::Remove,
        Icon::Lock,
        Icon::Unlock,
        Icon::Notification,
        Icon::CaseFile,
        Icon::Target,
        Icon::Payout,
        Icon::Reputation,
        Icon::Notoriety,
        Icon::Heat,
        Icon::Safehouse,
        Icon::Payroll,
        Icon::Doctor,
        Icon::Fence,
        Icon::Bail,
        Icon::Retire,
        Icon::Door,
        Icon::Dice,
        Icon::Advance,
        Icon::NewCampaign,
        Icon::Quieter,
        Icon::Louder,
        Icon::Hints,
        Icon::Change,
        Icon::Mute,
        Icon::Unmute,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Icon::Monogram => "monogram",
            Icon::Crew => "crew",
            Icon::Board => "board",
            Icon::Outfitter => "outfitter",
            Icon::LastJob => "last_job",
            Icon::Records => "records",
            Icon::Save => "save",
            Icon::Load => "load",
            Icon::Delete => "delete",
            Icon::Settings => "settings",
            Icon::Sound => "sound",
            Icon::Music => "music",
            Icon::Fullscreen => "fullscreen",
            Icon::Help => "help",
            Icon::Close => "close",
            Icon::Back => "back",
            Icon::Warning => "warning",
            Icon::Info => "info",
            Icon::Success => "success",
            Icon::Danger => "danger",
            Icon::Search => "search",
            Icon::Filter => "filter",
            Icon::Sort => "sort",
            Icon::Add => "add",
            Icon::Remove => "remove",
            Icon::Lock => "lock",
            Icon::Unlock => "unlock",
            Icon::Notification => "notification",
            Icon::CaseFile => "case_file",
            Icon::Target => "target",
            Icon::Payout => "payout",
            Icon::Reputation => "reputation",
            Icon::Notoriety => "notoriety",
            Icon::Heat => "heat",
            Icon::Safehouse => "safehouse",
            Icon::Payroll => "payroll",
            Icon::Doctor => "doctor",
            Icon::Fence => "fence",
            Icon::Bail => "bail",
            Icon::Retire => "retire",
            Icon::Door => "door",
            Icon::Dice => "dice",
            Icon::Advance => "advance",
            Icon::NewCampaign => "new_campaign",
            Icon::Quieter => "quieter",
            Icon::Louder => "louder",
            Icon::Hints => "hints",
            Icon::Change => "change",
            Icon::Mute => "mute",
            Icon::Unmute => "unmute",
        }
    }
}

pub fn draw_icon(icon: Icon, rect: Rect, color: Color) {
    let scale = (rect.w.min(rect.h) / GRID).max(0.1);
    let origin = vec2(
        rect.x + (rect.w - GRID * scale) * 0.5,
        rect.y + (rect.h - GRID * scale) * 0.5,
    );
    let p = |x: f32, y: f32| vec2(origin.x + x * scale, origin.y + y * scale);
    let line = |a: (f32, f32), b: (f32, f32), width: f32| {
        draw_line(
            p(a.0, a.1).x,
            p(a.0, a.1).y,
            p(b.0, b.1).x,
            p(b.0, b.1).y,
            width * scale,
            color,
        );
    };
    let circle = |x: f32, y: f32, radius: f32| {
        draw_circle_lines(p(x, y).x, p(x, y).y, radius * scale, 1.5 * scale, color);
    };
    let filled_circle = |x: f32, y: f32, radius: f32| {
        draw_circle(p(x, y).x, p(x, y).y, radius * scale, color);
    };
    let square = |x: f32, y: f32, width: f32, height: f32| {
        draw_rectangle_lines(
            p(x, y).x,
            p(x, y).y,
            width * scale,
            height * scale,
            1.5 * scale,
            color,
        );
    };
    let plus = || {
        line((6.0, 12.0), (18.0, 12.0), 1.8);
        line((12.0, 6.0), (12.0, 18.0), 1.8);
    };
    let x_mark = || {
        line((6.5, 6.5), (17.5, 17.5), 1.8);
        line((17.5, 6.5), (6.5, 17.5), 1.8);
    };

    match icon {
        Icon::Monogram => {
            square(2.5, 2.5, 19.0, 19.0);
            line((7.0, 17.0), (7.0, 7.0), 2.0);
            line((7.0, 7.0), (16.5, 7.0), 2.0);
            line((7.0, 12.0), (14.0, 12.0), 2.0);
            line((14.0, 12.0), (17.0, 17.0), 2.0);
        }
        Icon::Crew => {
            circle(8.0, 8.0, 3.0);
            circle(16.0, 8.0, 3.0);
            line((3.5, 19.0), (12.5, 19.0), 1.6);
            line((11.5, 19.0), (20.5, 19.0), 1.6);
            line((5.0, 13.0), (11.0, 13.0), 1.6);
            line((13.0, 13.0), (19.0, 13.0), 1.6);
        }
        Icon::Board | Icon::CaseFile => {
            square(3.0, 4.0, 18.0, 16.0);
            line((6.0, 8.0), (18.0, 8.0), 1.4);
            line((6.0, 12.0), (15.0, 12.0), 1.4);
            line((6.0, 16.0), (12.0, 16.0), 1.4);
            line((16.0, 14.0), (19.0, 17.0), 1.6);
            circle(15.0, 13.0, 2.5);
        }
        Icon::Outfitter | Icon::Fence => {
            line((4.0, 18.0), (20.0, 18.0), 1.8);
            line((6.0, 18.0), (8.0, 7.0), 1.8);
            line((18.0, 18.0), (16.0, 7.0), 1.8);
            line((8.0, 7.0), (16.0, 7.0), 1.8);
            line((9.0, 11.0), (15.0, 11.0), 1.5);
        }
        Icon::LastJob | Icon::Records => {
            square(3.0, 4.0, 18.0, 16.0);
            line((6.0, 16.0), (9.0, 13.0), 1.8);
            line((9.0, 13.0), (12.0, 15.0), 1.8);
            line((12.0, 15.0), (18.0, 8.0), 1.8);
            line((6.0, 8.0), (11.0, 8.0), 1.4);
        }
        Icon::Save => {
            square(3.0, 3.0, 18.0, 18.0);
            square(7.0, 4.5, 8.0, 5.5);
            square(7.0, 13.0, 10.0, 5.0);
        }
        Icon::Load => {
            square(4.0, 4.0, 16.0, 16.0);
            line((12.0, 7.0), (12.0, 16.0), 1.8);
            line((8.0, 12.0), (12.0, 16.0), 1.8);
            line((16.0, 12.0), (12.0, 16.0), 1.8);
        }
        Icon::Delete => {
            square(7.0, 7.0, 10.0, 13.0);
            line((5.0, 5.0), (19.0, 5.0), 1.8);
            line((10.0, 3.0), (14.0, 3.0), 1.8);
            line((10.0, 10.0), (10.0, 16.0), 1.2);
            line((14.0, 10.0), (14.0, 16.0), 1.2);
        }
        Icon::Settings => {
            circle(12.0, 12.0, 4.0);
            for angle in [0.0_f32, 1.57, 3.14, 4.71] {
                line(
                    (12.0 + angle.cos() * 6.0, 12.0 + angle.sin() * 6.0),
                    (12.0 + angle.cos() * 9.0, 12.0 + angle.sin() * 9.0),
                    2.0,
                );
            }
        }
        Icon::Sound | Icon::Music | Icon::Mute | Icon::Unmute => {
            square(4.0, 9.0, 4.0, 6.0);
            line((8.0, 9.0), (13.0, 5.0), 1.8);
            line((8.0, 15.0), (13.0, 19.0), 1.8);
            line((13.0, 5.0), (13.0, 19.0), 1.8);
            if matches!(icon, Icon::Mute) {
                x_mark();
            } else if !matches!(icon, Icon::Sound) {
                line((16.0, 9.0), (19.0, 12.0), 1.5);
                line((19.0, 12.0), (16.0, 15.0), 1.5);
            }
        }
        Icon::Fullscreen => {
            line((4.0, 9.0), (4.0, 4.0), 1.8);
            line((4.0, 4.0), (9.0, 4.0), 1.8);
            line((15.0, 4.0), (20.0, 4.0), 1.8);
            line((20.0, 4.0), (20.0, 9.0), 1.8);
            line((4.0, 15.0), (4.0, 20.0), 1.8);
            line((4.0, 20.0), (9.0, 20.0), 1.8);
            line((15.0, 20.0), (20.0, 20.0), 1.8);
            line((20.0, 20.0), (20.0, 15.0), 1.8);
        }
        Icon::Help => {
            circle(12.0, 12.0, 8.0);
            line((9.5, 9.0), (11.0, 7.5), 1.7);
            line((11.0, 7.5), (14.0, 7.5), 1.7);
            line((14.0, 7.5), (15.5, 9.0), 1.7);
            line((15.5, 9.0), (12.0, 13.0), 1.7);
            filled_circle(12.0, 17.0, 1.0);
        }
        Icon::Close | Icon::Danger => x_mark(),
        Icon::Back => {
            line((18.0, 12.0), (6.0, 12.0), 1.8);
            line((6.0, 12.0), (11.0, 7.0), 1.8);
            line((6.0, 12.0), (11.0, 17.0), 1.8);
        }
        Icon::Warning => {
            line((12.0, 3.0), (21.0, 20.0), 1.8);
            line((21.0, 20.0), (3.0, 20.0), 1.8);
            line((3.0, 20.0), (12.0, 3.0), 1.8);
            line((12.0, 8.0), (12.0, 14.0), 1.8);
            filled_circle(12.0, 17.0, 1.0);
        }
        Icon::Info => {
            circle(12.0, 12.0, 8.0);
            filled_circle(12.0, 8.0, 1.0);
            line((12.0, 11.0), (12.0, 17.0), 1.8);
        }
        Icon::Success => {
            circle(12.0, 12.0, 8.0);
            line((7.0, 12.0), (10.5, 15.5), 1.8);
            line((10.5, 15.5), (17.0, 8.5), 1.8);
        }
        Icon::Search => {
            circle(10.0, 10.0, 5.0);
            line((14.0, 14.0), (19.0, 19.0), 2.0);
        }
        Icon::Filter => {
            line((4.0, 5.0), (20.0, 5.0), 1.8);
            line((7.0, 12.0), (17.0, 12.0), 1.8);
            line((10.0, 19.0), (14.0, 19.0), 1.8);
        }
        Icon::Sort => {
            line((7.0, 5.0), (7.0, 19.0), 1.8);
            line((4.0, 8.0), (7.0, 5.0), 1.8);
            line((10.0, 5.0), (17.0, 5.0), 1.8);
            line((10.0, 12.0), (15.0, 12.0), 1.8);
            line((10.0, 19.0), (13.0, 19.0), 1.8);
        }
        Icon::Add => plus(),
        Icon::Remove => line((6.0, 12.0), (18.0, 12.0), 1.8),
        Icon::Lock => {
            square(5.0, 10.0, 14.0, 10.0);
            circle(12.0, 10.0, 5.0);
            line((12.0, 13.0), (12.0, 16.0), 1.5);
        }
        Icon::Unlock => {
            square(5.0, 10.0, 14.0, 10.0);
            line((8.0, 10.0), (8.0, 7.0), 1.8);
            line((8.0, 7.0), (11.0, 5.0), 1.8);
            line((11.0, 5.0), (15.0, 7.0), 1.8);
        }
        Icon::Notification => {
            line((7.0, 17.0), (17.0, 17.0), 1.8);
            line((8.0, 17.0), (8.0, 10.0), 1.8);
            line((8.0, 10.0), (12.0, 6.0), 1.8);
            line((12.0, 6.0), (16.0, 10.0), 1.8);
            line((16.0, 10.0), (16.0, 17.0), 1.8);
            filled_circle(12.0, 20.0, 1.2);
        }
        Icon::Target => {
            circle(12.0, 12.0, 8.0);
            circle(12.0, 12.0, 4.0);
            filled_circle(12.0, 12.0, 1.5);
        }
        Icon::Payout => {
            circle(12.0, 12.0, 8.0);
            line((12.0, 7.0), (12.0, 17.0), 1.5);
            line((9.0, 9.0), (15.0, 9.0), 1.5);
            line((9.0, 15.0), (15.0, 15.0), 1.5);
        }
        Icon::Reputation => {
            line((12.0, 3.0), (14.5, 8.5), 1.8);
            line((14.5, 8.5), (20.0, 9.0), 1.8);
            line((20.0, 9.0), (15.5, 13.0), 1.8);
            line((15.5, 13.0), (17.0, 19.0), 1.8);
            line((17.0, 19.0), (12.0, 15.5), 1.8);
            line((12.0, 15.5), (7.0, 19.0), 1.8);
            line((7.0, 19.0), (8.5, 13.0), 1.8);
            line((8.5, 13.0), (4.0, 9.0), 1.8);
            line((4.0, 9.0), (9.5, 8.5), 1.8);
        }
        Icon::Notoriety | Icon::Heat => {
            line((12.0, 3.0), (18.0, 9.0), 1.8);
            line((18.0, 9.0), (15.0, 20.0), 1.8);
            line((15.0, 20.0), (9.0, 20.0), 1.8);
            line((9.0, 20.0), (6.0, 9.0), 1.8);
            line((6.0, 9.0), (12.0, 3.0), 1.8);
            line((9.0, 13.0), (15.0, 13.0), 1.5);
        }
        Icon::Safehouse => {
            line((3.0, 11.0), (12.0, 4.0), 1.8);
            line((12.0, 4.0), (21.0, 11.0), 1.8);
            square(6.0, 11.0, 12.0, 9.0);
            line((10.0, 20.0), (10.0, 15.0), 1.5);
            line((14.0, 20.0), (14.0, 15.0), 1.5);
        }
        Icon::Payroll => {
            square(4.0, 6.0, 16.0, 13.0);
            line((7.0, 10.0), (17.0, 10.0), 1.5);
            line((7.0, 14.0), (13.0, 14.0), 1.5);
            line((7.0, 17.0), (10.0, 17.0), 1.5);
        }
        Icon::Doctor => {
            square(5.0, 5.0, 14.0, 14.0);
            line((12.0, 8.0), (12.0, 16.0), 2.0);
            line((8.0, 12.0), (16.0, 12.0), 2.0);
        }
        Icon::Bail => {
            circle(12.0, 12.0, 8.0);
            line((8.0, 12.0), (16.0, 12.0), 1.8);
            line((12.0, 8.0), (12.0, 16.0), 1.8);
        }
        Icon::Retire => {
            line((4.0, 17.0), (20.0, 17.0), 1.8);
            line((6.0, 17.0), (6.0, 8.0), 1.8);
            line((6.0, 8.0), (12.0, 4.0), 1.8);
            line((12.0, 4.0), (18.0, 8.0), 1.8);
            line((18.0, 8.0), (18.0, 17.0), 1.8);
            line((12.0, 9.0), (12.0, 17.0), 1.5);
        }
        Icon::Door => {
            square(6.0, 3.0, 12.0, 18.0);
            line((9.0, 12.0), (10.0, 12.0), 1.8);
            line((18.0, 12.0), (21.0, 12.0), 1.8);
            line((18.0, 12.0), (16.0, 10.0), 1.8);
            line((18.0, 12.0), (16.0, 14.0), 1.8);
        }
        Icon::Dice => {
            square(4.0, 4.0, 16.0, 16.0);
            filled_circle(8.0, 8.0, 1.2);
            filled_circle(16.0, 16.0, 1.2);
            filled_circle(12.0, 12.0, 1.2);
        }
        Icon::Advance => {
            line((4.0, 12.0), (19.0, 12.0), 1.8);
            line((14.0, 7.0), (19.0, 12.0), 1.8);
            line((14.0, 17.0), (19.0, 12.0), 1.8);
            line((4.0, 7.0), (4.0, 17.0), 1.8);
        }
        Icon::NewCampaign => {
            square(4.0, 4.0, 16.0, 16.0);
            plus();
        }
        Icon::Quieter => {
            line((5.0, 12.0), (19.0, 12.0), 1.8);
            line((5.0, 12.0), (9.0, 8.0), 1.8);
            line((5.0, 12.0), (9.0, 16.0), 1.8);
        }
        Icon::Louder => {
            line((5.0, 12.0), (19.0, 12.0), 1.8);
            line((19.0, 12.0), (15.0, 8.0), 1.8);
            line((19.0, 12.0), (15.0, 16.0), 1.8);
        }
        Icon::Hints => {
            circle(12.0, 10.0, 6.0);
            line((9.0, 16.0), (15.0, 16.0), 1.8);
            line((10.0, 19.0), (14.0, 19.0), 1.8);
            line((12.0, 4.0), (12.0, 2.0), 1.5);
        }
        Icon::Change => {
            line((5.0, 8.0), (18.0, 8.0), 1.8);
            line((15.0, 5.0), (18.0, 8.0), 1.8);
            line((19.0, 16.0), (6.0, 16.0), 1.8);
            line((9.0, 13.0), (6.0, 16.0), 1.8);
        }
    }
}

#[cfg(test)]
mod tests;
