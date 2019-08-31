use super::app::{ActiveBlock, App, EventLoop, SearchResultBlock, SongTableContext};
use failure::err_msg;
use rspotify::spotify::model::offset::for_position;
use rspotify::spotify::model::track::FullTrack;
use rspotify::spotify::senum::Country;

use termion::event::Key;

fn down_event(key: Key) -> bool {
    match key {
        Key::Down | Key::Char('j') | Key::Ctrl('n') => true,
        _ => false,
    }
}

fn up_event(key: Key) -> bool {
    match key {
        Key::Up | Key::Char('k') | Key::Ctrl('p') => true,
        _ => false,
    }
}

fn left_event(key: Key) -> bool {
    match key {
        Key::Left | Key::Char('h') => true,
        _ => false,
    }
}

fn right_event(key: Key) -> bool {
    match key {
        Key::Right | Key::Char('l') => true,
        _ => false,
    }
}

fn on_down_press_handler<T>(selection_data: &[T], selection_index: Option<usize>) -> usize {
    match selection_index {
        Some(selection_index) => {
            if !selection_data.is_empty() {
                let next_index = selection_index + 1;
                if next_index > selection_data.len() - 1 {
                    return 0;
                } else {
                    return next_index;
                }
            }
            0
        }
        None => 0,
    }
}

fn on_up_press_handler<T>(selection_data: &[T], selection_index: Option<usize>) -> usize {
    match selection_index {
        Some(selection_index) => {
            if !selection_data.is_empty() {
                if selection_index > 0 {
                    return selection_index - 1;
                } else {
                    return selection_data.len() - 1;
                }
            }
            0
        }
        None => 0,
    }
}

pub fn input_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Ctrl('u') => {
            app.input = String::new();
            None
        }
        Key::Esc => {
            app.active_block = ActiveBlock::MyPlaylists;
            None
        }
        Key::Char('\n') => {
            if let Some(spotify) = &app.spotify {
                // TODO: This should be definable by the user
                let country = Some(Country::UnitedKingdom);

                let result = spotify
                    .search_track(&app.input, app.small_search_limit, 0, country)
                    // TODO handle the error properly
                    .expect("Failed to fetch spotify tracks");

                app.songs_for_table = result.tracks.items.clone();
                app.search_results.tracks = Some(result);

                // On searching for a track, clear the playlist selection
                app.selected_playlist_index = None;
                app.active_block = ActiveBlock::SearchResultBlock;

                // Can I run these functions in parellel?
                let result = spotify
                    .search_artist(
                        &app.input,
                        app.small_search_limit,
                        0,
                        Some(Country::UnitedKingdom),
                    )
                    .expect("Failed to fetch artists");
                app.search_results.artists = Some(result);

                let result = spotify
                    .search_album(
                        &app.input,
                        app.small_search_limit,
                        0,
                        Some(Country::UnitedKingdom),
                    )
                    .expect("Failed to fetch albums");
                app.search_results.albums = Some(result);

                let result = spotify
                    .search_playlist(
                        &app.input,
                        app.small_search_limit,
                        0,
                        Some(Country::UnitedKingdom),
                    )
                    .expect("Failed to fetch playlists");
                app.search_results.playlists = Some(result);
            }
            None
        }
        Key::Char(c) => {
            app.input.push(c);
            None
        }
        Key::Backspace => {
            app.input.pop();
            None
        }
        _ => None,
    }
}

pub fn playlist_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Char('d') => {
            handle_get_devices(app);
            None
        }
        Key::Char('?') => {
            app.active_block = ActiveBlock::HelpMenu;
            None
        }
        k if right_event(k) => {
            if app.selected_playlist_index.is_some() {
                app.active_block = ActiveBlock::SongTable;
            } else if !app.input.is_empty() {
                app.active_block = ActiveBlock::SearchResultBlock;
            }
            None
        }
        k if down_event(k) => match &app.playlists {
            Some(p) => {
                if let Some(selected_playlist_index) = app.selected_playlist_index {
                    let next_index = on_down_press_handler(&p.items, Some(selected_playlist_index));
                    app.selected_playlist_index = Some(next_index);
                }
                None
            }
            None => None,
        },
        k if up_event(k) => match &app.playlists {
            Some(p) => {
                let next_index = on_up_press_handler(&p.items, app.selected_playlist_index);
                app.selected_playlist_index = Some(next_index);
                None
            }
            None => None,
        },
        Key::Char('/') => {
            app.active_block = ActiveBlock::Input;
            None
        }
        Key::Char('\n') => match (&app.playlists, &app.selected_playlist_index) {
            (Some(playlists), Some(selected_playlist_index)) => {
                app.song_table_context = Some(SongTableContext::MyPlaylists);
                if let Some(selected_playlist) =
                    playlists.items.get(selected_playlist_index.to_owned())
                {
                    let playlist_id = selected_playlist.id.to_owned();
                    get_playlist_tracks(playlist_id, app);
                }
                None
            }
            _ => None,
        },
        _ => None,
    }
}

pub fn song_table_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Char('d') => {
            handle_get_devices(app);
            None
        }
        k if left_event(k) => {
            app.active_block = ActiveBlock::MyPlaylists;
            None
        }
        k if down_event(k) => {
            let next_index =
                on_down_press_handler(&app.songs_for_table, Some(app.select_song_index));
            app.select_song_index = next_index;
            None
        }
        Key::Char('?') => {
            app.active_block = ActiveBlock::HelpMenu;
            None
        }
        k if up_event(k) => {
            let next_index = on_up_press_handler(&app.songs_for_table, Some(app.select_song_index));
            app.select_song_index = next_index;
            None
        }
        Key::Char('/') => {
            app.active_block = ActiveBlock::Input;
            None
        }
        Key::Char('\n') => {
            match &app.song_table_context {
                Some(context) => match context {
                    SongTableContext::MyPlaylists => {
                        if let Some(track) = app.songs_for_table.get(app.select_song_index) {
                            let context_uri = match (&app.selected_playlist_index, &app.playlists) {
                                (Some(selected_playlist_index), Some(playlists)) => {
                                    if let Some(selected_playlist) =
                                        playlists.items.get(selected_playlist_index.to_owned())
                                    {
                                        Some(selected_playlist.uri.to_owned())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };

                            match start_playback(
                                app,
                                context_uri,
                                None,
                                Some(app.select_song_index),
                            ) {
                                Ok(_r) => {
                                    app.current_playing_song = Some(track.to_owned());
                                }
                                Err(e) => {
                                    app.active_block = ActiveBlock::ApiError;
                                    app.api_error = e.to_string();
                                }
                            };
                        };
                    }
                    SongTableContext::AlbumSearch => {}
                    SongTableContext::SongSearch => {}
                    SongTableContext::ArtistSearch => {}
                    SongTableContext::PlaylistSearch => {
                        if let Some(track) = app.songs_for_table.get(app.select_song_index) {
                            let context_uri = match (
                                &app.search_results.selected_playlists_index,
                                &app.search_results.playlists,
                            ) {
                                (Some(selected_playlist_index), Some(playlist_result)) => {
                                    if let Some(selected_playlist) = playlist_result
                                        .playlists
                                        .items
                                        .get(selected_playlist_index.to_owned())
                                    {
                                        Some(selected_playlist.uri.to_owned())
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            };

                            match start_playback(
                                app,
                                context_uri,
                                None,
                                Some(app.select_song_index),
                            ) {
                                Ok(_r) => {
                                    app.current_playing_song = Some(track.to_owned());
                                }
                                Err(e) => {
                                    app.active_block = ActiveBlock::ApiError;
                                    app.api_error = e.to_string();
                                }
                            };
                        };
                    }
                },
                None => {}
            };

            None
        }
        _ => None,
    }
}

pub fn help_menu_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Esc => {
            app.active_block = ActiveBlock::MyPlaylists;
            None
        }
        Key::Char('d') => {
            handle_get_devices(app);
            None
        }
        _ => None,
    }
}

pub fn api_error_menu_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Esc => {
            app.active_block = ActiveBlock::MyPlaylists;
            None
        }
        Key::Char('d') => {
            handle_get_devices(app);
            None
        }
        _ => None,
    }
}

pub fn search_results_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Char('d') => {
            handle_get_devices(app);
            None
        }
        Key::Char('?') => {
            app.active_block = ActiveBlock::HelpMenu;
            None
        }
        Key::Esc => {
            app.search_results.selected_block = SearchResultBlock::Empty;
            None
        }
        k if down_event(k) => {
            if app.search_results.selected_block != SearchResultBlock::Empty {
                // Start selecting within the selected block
                match app.search_results.selected_block {
                    SearchResultBlock::AlbumSearch => {
                        if let Some(result) = &app.search_results.albums {
                            let next_index = on_down_press_handler(
                                &result.albums.items,
                                app.search_results.selected_album_index,
                            );
                            app.search_results.selected_album_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::SongSearch => {
                        if let Some(result) = &app.search_results.tracks {
                            let next_index = on_down_press_handler(
                                &result.tracks.items,
                                app.search_results.selected_tracks_index,
                            );
                            app.search_results.selected_tracks_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::ArtistSearch => {
                        if let Some(result) = &app.search_results.artists {
                            let next_index = on_down_press_handler(
                                &result.artists.items,
                                app.search_results.selected_artists_index,
                            );
                            app.search_results.selected_artists_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::PlaylistSearch => {
                        if let Some(result) = &app.search_results.playlists {
                            let next_index = on_down_press_handler(
                                &result.playlists.items,
                                app.search_results.selected_playlists_index,
                            );
                            app.search_results.selected_playlists_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::Empty => {}
                }
            } else {
                match app.search_results.hovered_block {
                    SearchResultBlock::AlbumSearch => {
                        app.search_results.hovered_block = SearchResultBlock::SongSearch;
                    }
                    SearchResultBlock::SongSearch => {
                        app.search_results.hovered_block = SearchResultBlock::AlbumSearch;
                    }
                    SearchResultBlock::ArtistSearch => {
                        app.search_results.hovered_block = SearchResultBlock::PlaylistSearch;
                    }
                    SearchResultBlock::PlaylistSearch => {
                        app.search_results.hovered_block = SearchResultBlock::ArtistSearch;
                    }
                    SearchResultBlock::Empty => {}
                }
            }
            None
        }
        k if up_event(k) => {
            if app.search_results.selected_block != SearchResultBlock::Empty {
                // Start selecting within the selected block
                match app.search_results.selected_block {
                    SearchResultBlock::AlbumSearch => {
                        if let Some(result) = &app.search_results.albums {
                            let next_index = on_up_press_handler(
                                &result.albums.items,
                                app.search_results.selected_album_index,
                            );
                            app.search_results.selected_album_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::SongSearch => {
                        if let Some(result) = &app.search_results.tracks {
                            let next_index = on_up_press_handler(
                                &result.tracks.items,
                                app.search_results.selected_tracks_index,
                            );
                            app.search_results.selected_tracks_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::ArtistSearch => {
                        if let Some(result) = &app.search_results.artists {
                            let next_index = on_up_press_handler(
                                &result.artists.items,
                                app.search_results.selected_artists_index,
                            );
                            app.search_results.selected_artists_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::PlaylistSearch => {
                        if let Some(result) = &app.search_results.playlists {
                            let next_index = on_up_press_handler(
                                &result.playlists.items,
                                app.search_results.selected_playlists_index,
                            );
                            app.search_results.selected_playlists_index = Some(next_index);
                        }
                    }
                    SearchResultBlock::Empty => {}
                }
            } else {
                match app.search_results.hovered_block {
                    SearchResultBlock::AlbumSearch => {
                        app.search_results.hovered_block = SearchResultBlock::SongSearch;
                    }
                    SearchResultBlock::SongSearch => {
                        app.search_results.hovered_block = SearchResultBlock::AlbumSearch;
                    }
                    SearchResultBlock::ArtistSearch => {
                        app.search_results.hovered_block = SearchResultBlock::PlaylistSearch;
                    }
                    SearchResultBlock::PlaylistSearch => {
                        app.search_results.hovered_block = SearchResultBlock::ArtistSearch;
                    }
                    SearchResultBlock::Empty => {}
                }
            }
            None
        }
        k if left_event(k) => {
            app.search_results.selected_block = SearchResultBlock::Empty;
            match app.search_results.hovered_block {
                SearchResultBlock::AlbumSearch => {
                    app.active_block = ActiveBlock::MyPlaylists;
                }
                SearchResultBlock::SongSearch => {
                    app.active_block = ActiveBlock::MyPlaylists;
                }
                SearchResultBlock::ArtistSearch => {
                    app.search_results.hovered_block = SearchResultBlock::SongSearch;
                }
                SearchResultBlock::PlaylistSearch => {
                    app.search_results.hovered_block = SearchResultBlock::AlbumSearch;
                }
                SearchResultBlock::Empty => {}
            }
            None
        }
        k if right_event(k) => {
            app.search_results.selected_block = SearchResultBlock::Empty;
            match app.search_results.hovered_block {
                SearchResultBlock::AlbumSearch => {
                    app.search_results.hovered_block = SearchResultBlock::PlaylistSearch;
                }
                SearchResultBlock::SongSearch => {
                    app.search_results.hovered_block = SearchResultBlock::ArtistSearch;
                }
                SearchResultBlock::ArtistSearch => {
                    app.search_results.hovered_block = SearchResultBlock::SongSearch;
                }
                SearchResultBlock::PlaylistSearch => {
                    app.search_results.hovered_block = SearchResultBlock::AlbumSearch;
                }
                SearchResultBlock::Empty => {}
            }
            None
        }
        // Handle pressing enter when block is selected to start playing track
        Key::Char('\n') => {
            if app.search_results.selected_block != SearchResultBlock::Empty {
                match app.search_results.selected_block {
                    SearchResultBlock::AlbumSearch => {
                        if let (Some(index), Some(albums_result)) = (
                            app.search_results.selected_album_index,
                            &app.search_results.albums,
                        ) {
                            if let Some(album) = albums_result.albums.items.get(index) {
                                // TODO: Go to album table
                            };
                        }
                    }
                    SearchResultBlock::SongSearch => {
                        if let (Some(index), Some(result)) = (
                            app.search_results.selected_tracks_index,
                            &app.search_results.tracks,
                        ) {
                            if let Some(track) = result.tracks.items.get(index) {
                                match start_playback(
                                    app,
                                    None,
                                    Some(vec![track.uri.to_owned()]),
                                    Some(0),
                                ) {
                                    Ok(_r) => {
                                        app.current_playing_song = Some(track.to_owned());
                                    }
                                    Err(e) => {
                                        app.active_block = ActiveBlock::ApiError;
                                        app.api_error = e.to_string();
                                    }
                                };
                            };
                        };
                    }
                    SearchResultBlock::ArtistSearch => {
                        // TODO: Go to artist view (not yet implemented)
                    }
                    SearchResultBlock::PlaylistSearch => {
                        if let (Some(index), Some(playlists_result)) = (
                            app.search_results.selected_playlists_index,
                            &app.search_results.playlists,
                        ) {
                            if let Some(playlist) = playlists_result.playlists.items.get(index) {
                                // Go to playlist tracks table
                                app.song_table_context = Some(SongTableContext::PlaylistSearch);
                                let playlist_id = playlist.id.to_owned();
                                get_playlist_tracks(playlist_id, app);
                            };
                        }
                    }
                    SearchResultBlock::Empty => {}
                };
            } else {
                match app.search_results.hovered_block {
                    SearchResultBlock::AlbumSearch => {
                        app.search_results.selected_album_index = Some(0);
                        app.search_results.selected_block = SearchResultBlock::AlbumSearch;
                    }
                    SearchResultBlock::SongSearch => {
                        app.search_results.selected_tracks_index = Some(0);
                        app.search_results.selected_block = SearchResultBlock::SongSearch;
                    }
                    SearchResultBlock::ArtistSearch => {
                        app.search_results.selected_artists_index = Some(0);
                        app.search_results.selected_block = SearchResultBlock::ArtistSearch;
                    }
                    SearchResultBlock::PlaylistSearch => {
                        app.search_results.selected_playlists_index = Some(0);
                        app.search_results.selected_block = SearchResultBlock::PlaylistSearch;
                    }
                    SearchResultBlock::Empty => {}
                };
            }
            None
        }
        // Add `s` to "see more" on each option
        _ => None,
    }
}

pub fn select_device_handler(key: Key, app: &mut App) -> Option<EventLoop> {
    match key {
        Key::Char('q') | Key::Ctrl('c') => Some(EventLoop::Exit),
        Key::Esc => {
            app.active_block = ActiveBlock::MyPlaylists;
            None
        }
        k if down_event(k) => match &app.devices {
            Some(p) => {
                if let Some(selected_device_index) = app.selected_device_index {
                    let next_index = on_down_press_handler(&p.devices, Some(selected_device_index));
                    app.selected_device_index = Some(next_index);
                }
                None
            }
            None => None,
        },
        k if up_event(k) => match &app.devices {
            Some(p) => {
                if let Some(selected_device_index) = app.selected_device_index {
                    let next_index = on_up_press_handler(&p.devices, Some(selected_device_index));
                    app.selected_device_index = Some(next_index);
                }
                None
            }
            None => None,
        },
        Key::Char('\n') => match (&app.devices, app.selected_device_index) {
            (Some(devices), Some(index)) => {
                if let Some(device) = devices.devices.get(index) {
                    app.device_id = Some(device.id.to_owned());
                    app.active_block = ActiveBlock::MyPlaylists;
                }
                None
            }
            _ => None,
        },
        _ => None,
    }
}

fn handle_get_devices(app: &mut App) {
    if let Some(spotify) = &app.spotify {
        if let Ok(result) = spotify.device() {
            app.active_block = ActiveBlock::SelectDevice;

            if !result.devices.is_empty() {
                app.devices = Some(result);
                // Select the first device in the list
                app.selected_device_index = Some(0);
            }
        }
    }
}

fn start_playback(
    app: &App,
    context_uri: Option<String>,
    uris: Option<Vec<String>>,
    offset: Option<usize>,
) -> Result<(), failure::Error> {
    let (uris, context_uri) = if context_uri.is_some() {
        (None, context_uri)
    } else {
        (uris, None)
    };

    let offset = match offset {
        Some(o) => o,
        None => 0,
    };

    match &app.device_id {
        Some(device_id) => match &app.spotify {
            Some(spotify) => spotify.start_playback(
                Some(device_id.to_string()),
                context_uri,
                uris,
                for_position(offset as u32),
            ),
            None => Err(err_msg("Spotify is not ready to be used".to_string())),
        },
        None => Err(err_msg("No device_id selected")),
    }
}

fn get_playlist_tracks(playlist_id: String, app: &mut App) {
    match &app.spotify {
        Some(spotify) => {
            if let Ok(playlist_tracks) = spotify.user_playlist_tracks(
                "spotify",
                &playlist_id,
                None,
                Some(app.large_search_limit),
                None,
                None,
            ) {
                app.songs_for_table = playlist_tracks
                    .items
                    .clone()
                    .into_iter()
                    .map(|item| item.track)
                    .collect::<Vec<FullTrack>>();

                app.playlist_tracks = playlist_tracks.items;
                app.active_block = ActiveBlock::SongTable;
            };
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_input_handler_quits() {
        let mut app = App::new();

        let result = input_handler(Key::Char('q'), &mut app);
        assert_eq!(result, Some(EventLoop::Exit));

        let result = input_handler(Key::Ctrl('c'), &mut app);
        assert_eq!(result, Some(EventLoop::Exit));
    }

    #[test]
    fn test_input_handler_clear_input_on_ctrl_u() {
        let mut app = App::new();

        app.input = "My text".to_string();

        let result = input_handler(Key::Ctrl('u'), &mut app);

        assert_eq!(result, None);
        assert_eq!(app.input, "".to_string());
    }

    #[test]
    fn test_input_handler_esc_back_to_playlist() {
        let mut app = App::new();

        let result = input_handler(Key::Esc, &mut app);

        assert_eq!(result, None);
        assert_eq!(app.active_block, ActiveBlock::MyPlaylists);
    }

    #[test]
    fn test_input_handler_on_enter_text() {
        let mut app = App::new();

        app.input = "My tex".to_string();

        let result = input_handler(Key::Char('t'), &mut app);

        assert_eq!(result, None);
        assert_eq!(app.input, "My text".to_string());
    }

    #[test]
    fn test_input_handler_backspace() {
        let mut app = App::new();

        app.input = "My text".to_string();

        let result = input_handler(Key::Backspace, &mut app);

        assert_eq!(result, None);
        assert_eq!(app.input, "My tex".to_string());
    }

    #[test]
    fn test_playlist_handler_quit() {
        let mut app = App::new();

        let result = playlist_handler(Key::Char('q'), &mut app);
        assert_eq!(result, Some(EventLoop::Exit));

        let result = playlist_handler(Key::Ctrl('c'), &mut app);
        assert_eq!(result, Some(EventLoop::Exit));
    }

    #[test]
    fn test_playlist_handler_activate_help_menu() {
        let mut app = App::new();

        let result = playlist_handler(Key::Char('?'), &mut app);
        assert_eq!(result, None);
        assert_eq!(app.active_block, ActiveBlock::HelpMenu);
    }

    #[test]
    fn test_on_down_press_handler() {
        let data = vec!["Choice 1", "Choice 2", "Choice 3"];

        let index = 0;
        let next_index = on_down_press_handler(&data, Some(index));

        assert_eq!(next_index, 1);

        // Selection wrap if on last item
        let index = data.len() - 1;
        let next_index = on_down_press_handler(&data, Some(index));
        assert_eq!(next_index, 0);
    }

    #[test]
    fn test_on_up_press_handler() {
        let data = vec!["Choice 1", "Choice 2", "Choice 3"];

        let index = data.len() - 1;
        let next_index = on_up_press_handler(&data, Some(index));

        assert_eq!(next_index, index - 1);

        // Selection wrap if on first item
        let index = 0;
        let next_index = on_up_press_handler(&data, Some(index));
        assert_eq!(next_index, data.len() - 1);
    }
}
