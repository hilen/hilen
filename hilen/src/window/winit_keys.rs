//! winit keys and buttons in the types of the engine, see
//! `ui/input/keyboard.rs`. Both key lists follow the W3C list, so 1 macro
//! maps every name they share and the compiler checks each one.

use winit::{
    event::{ElementState as WinitState, MouseButton as WinitButton},
    keyboard::{KeyCode as WinitCode, ModifiersState, NamedKey as WinitNamed},
};

use crate::ui::{Code, ElementState, Modifiers, MouseButton, NamedKey};

macro_rules! same_names {
    ($value:expr, $from:ident => $to:ident, [$($name:ident),* $(,)?], $($extra:tt)*) => {
        match $value {
            $($from::$name => Some($to::$name),)*
            $($extra)*
        }
    };
}

/// None for the space bar, which W3C has as the character " ", and for a
/// key winit added after this list. Super and Hyper are Meta now, the W3C
/// list keeps the old names only as legacy.
pub(crate) fn named_key(key: WinitNamed) -> Option<NamedKey> {
    same_names!(
        key,
        WinitNamed => NamedKey,
        [
        AVRInput, AVRPower, Accept, Again, AllCandidates, Alphanumeric, Alt, AltGraph, AppSwitch,
        ArrowDown, ArrowLeft, ArrowRight, ArrowUp, Attn, AudioBalanceLeft, AudioBalanceRight,
        AudioBassBoostDown, AudioBassBoostToggle, AudioBassBoostUp, AudioFaderFront, AudioFaderRear,
        AudioSurroundModeNext, AudioTrebleDown, AudioTrebleUp, AudioVolumeDown, AudioVolumeMute,
        AudioVolumeUp, Backspace, BrightnessDown, BrightnessUp, BrowserBack, BrowserFavorites,
        BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch, BrowserStop, Call, Camera,
        CameraFocus, Cancel, CapsLock, ChannelDown, ChannelUp, Clear, Close, ClosedCaptionToggle,
        CodeInput, ColorF0Red, ColorF1Green, ColorF2Yellow, ColorF3Blue, ColorF4Grey, ColorF5Brown,
        Compose, ContextMenu, Control, Convert, Copy, CrSel, Cut, DVR, Delete, Dimmer, DisplaySwap,
        Eisu, Eject, End, EndCall, Enter, EraseEof, Escape, ExSel, Execute, Exit, F1, F10, F11, F12,
        F13, F14, F15, F16, F17, F18, F19, F2, F20, F21, F22, F23, F24, F25, F26, F27, F28, F29, F3,
        F30, F31, F32, F33, F34, F35, F4, F5, F6, F7, F8, F9, FavoriteClear0, FavoriteClear1,
        FavoriteClear2, FavoriteClear3, FavoriteRecall0, FavoriteRecall1, FavoriteRecall2,
        FavoriteRecall3, FavoriteStore0, FavoriteStore1, FavoriteStore2, FavoriteStore3, FinalMode,
        Find, Fn, FnLock, GoBack, GoHome, GroupFirst, GroupLast, GroupNext, GroupPrevious, Guide,
        GuideNextDay, GuidePreviousDay, HangulMode, HanjaMode, Hankaku, HeadsetHook, Help,
        Hibernate, Hiragana, HiraganaKatakana, Home,  Info, Insert, InstantReplay, JunjaMode,
        KanaMode, KanjiMode, Katakana, Key11, Key12, LastNumberRedial, LaunchApplication1,
        LaunchApplication2, LaunchCalendar, LaunchContacts, LaunchMail, LaunchMediaPlayer,
        LaunchMusicPlayer, LaunchPhone, LaunchScreenSaver, LaunchSpreadsheet, LaunchWebBrowser,
        LaunchWebCam, LaunchWordProcessor, Link, ListProgram, LiveContent, Lock, LogOff,
        MailForward, MailReply, MailSend, MannerMode, MediaApps, MediaAudioTrack, MediaClose,
        MediaFastForward, MediaLast, MediaPause, MediaPlay, MediaPlayPause, MediaRecord,
        MediaRewind, MediaSkipBackward, MediaSkipForward, MediaStepBackward, MediaStepForward,
        MediaStop, MediaTopMenu, MediaTrackNext, MediaTrackPrevious, Meta, MicrophoneToggle,
        MicrophoneVolumeDown, MicrophoneVolumeMute, MicrophoneVolumeUp, ModeChange, NavigateIn,
        NavigateNext, NavigateOut, NavigatePrevious, New, NextCandidate, NextFavoriteChannel,
        NextUserProfile, NonConvert, Notification, NumLock, OnDemand, Open, PageDown, PageUp,
        Pairing, Paste, Pause, PinPDown, PinPMove, PinPToggle, PinPUp, Play, PlaySpeedDown,
        PlaySpeedReset, PlaySpeedUp, Power, PowerOff, PreviousCandidate, Print, PrintScreen,
        Process, Props, RandomToggle, RcLowBattery, RecordSpeedNext, Redo, RfBypass, Romaji,
        STBInput, STBPower, Save, ScanChannelsToggle, ScreenModeNext, ScrollLock, Select, Settings,
        Shift, SingleCandidate, Soft1, Soft2, Soft3, Soft4, SpeechCorrectionList, SpeechInputToggle,
        SpellCheck, SplitScreenToggle, Standby, Subtitle,  Symbol, SymbolLock, TV, TV3DMode,
        TVAntennaCable, TVAudioDescription, TVAudioDescriptionMixDown, TVAudioDescriptionMixUp,
        TVContentsMenu, TVDataService, TVInput, TVInputComponent1, TVInputComponent2,
        TVInputComposite1, TVInputComposite2, TVInputHDMI1, TVInputHDMI2, TVInputHDMI3,
        TVInputHDMI4, TVInputVGA1, TVMediaContext, TVNetwork, TVNumberEntry, TVPower,
        TVRadioService, TVSatellite, TVSatelliteBS, TVSatelliteCS, TVSatelliteToggle,
        TVTerrestrialAnalog, TVTerrestrialDigital, TVTimer, Tab, Teletext, Undo, VideoModeNext,
        VoiceDial, WakeUp, Wink, Zenkaku, ZenkakuHankaku, ZoomIn, ZoomOut, ZoomToggle,
        ],
        WinitNamed::Super | WinitNamed::Hyper => Some(NamedKey::Meta),
        _ => None,
    )
}

/// None for a code winit added after this list, and for the legacy codes
/// Meta, Hyper and Turbo of old keyboards.
pub(crate) fn code(code: WinitCode) -> Option<Code> {
    same_names!(
        code,
        WinitCode => Code,
        [
        Abort, Again, AltLeft, AltRight, ArrowDown, ArrowLeft, ArrowRight, ArrowUp, AudioVolumeDown,
        AudioVolumeMute, AudioVolumeUp, Backquote, Backslash, Backspace, BracketLeft, BracketRight,
        BrowserBack, BrowserFavorites, BrowserForward, BrowserHome, BrowserRefresh, BrowserSearch,
        BrowserStop, CapsLock, Comma, ContextMenu, ControlLeft, ControlRight, Convert, Copy, Cut,
        Delete, Digit0, Digit1, Digit2, Digit3, Digit4, Digit5, Digit6, Digit7, Digit8, Digit9,
        Eject, End, Enter, Equal, Escape, F1, F10, F11, F12, F13, F14, F15, F16, F17, F18, F19, F2,
        F20, F21, F22, F23, F24, F25, F26, F27, F28, F29, F3, F30, F31, F32, F33, F34, F35, F4, F5,
        F6, F7, F8, F9, Find, Fn, FnLock, Help, Hiragana, Home, Insert, IntlBackslash,
        IntlRo, IntlYen, KanaMode, Katakana, KeyA, KeyB, KeyC, KeyD, KeyE, KeyF, KeyG, KeyH, KeyI,
        KeyJ, KeyK, KeyL, KeyM, KeyN, KeyO, KeyP, KeyQ, KeyR, KeyS, KeyT, KeyU, KeyV, KeyW, KeyX,
        KeyY, KeyZ, Lang1, Lang2, Lang3, Lang4, Lang5, LaunchApp1, LaunchApp2, LaunchMail,
        MediaPlayPause, MediaSelect, MediaStop, MediaTrackNext, MediaTrackPrevious, Minus,
        NonConvert, NumLock, Numpad0, Numpad1, Numpad2, Numpad3, Numpad4, Numpad5, Numpad6, Numpad7,
        Numpad8, Numpad9, NumpadAdd, NumpadBackspace, NumpadClear, NumpadClearEntry, NumpadComma,
        NumpadDecimal, NumpadDivide, NumpadEnter, NumpadEqual, NumpadHash, NumpadMemoryAdd,
        NumpadMemoryClear, NumpadMemoryRecall, NumpadMemoryStore, NumpadMemorySubtract,
        NumpadMultiply, NumpadParenLeft, NumpadParenRight, NumpadStar, NumpadSubtract, Open,
        PageDown, PageUp, Paste, Pause, Period, Power, PrintScreen, Props, Quote, Resume,
        ScrollLock, Select, Semicolon, ShiftLeft, ShiftRight, Slash, Sleep, Space, Suspend, Tab,
        Undo, WakeUp,
        ],
        WinitCode::SuperLeft => Some(Code::MetaLeft),
        WinitCode::SuperRight => Some(Code::MetaRight),
        _ => None,
    )
}

pub(crate) fn modifiers(state: ModifiersState) -> Modifiers {
    let mut modifiers = Modifiers::empty();
    modifiers.set(Modifiers::SHIFT, state.shift_key());
    modifiers.set(Modifiers::CONTROL, state.control_key());
    modifiers.set(Modifiers::ALT, state.alt_key());
    modifiers.set(Modifiers::META, state.super_key());
    modifiers
}

pub(crate) fn mouse_button(button: WinitButton) -> MouseButton {
    match button {
        WinitButton::Left => MouseButton::Left,
        WinitButton::Right => MouseButton::Right,
        WinitButton::Middle => MouseButton::Middle,
        WinitButton::Back => MouseButton::Back,
        WinitButton::Forward => MouseButton::Forward,
        WinitButton::Other(other) => MouseButton::Other(other),
    }
}

pub(crate) fn element_state(state: WinitState) -> ElementState {
    match state {
        WinitState::Pressed => ElementState::Pressed,
        WinitState::Released => ElementState::Released,
    }
}
