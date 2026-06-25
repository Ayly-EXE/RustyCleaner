mod utils;

use std::collections::HashMap;
use std::fs;
use std::path::{PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use clap::Parser;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use fs_extra::dir::get_size;
use ratatui::Frame;
use ratatui::layout::Constraint::{Fill, Length};
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Clear, Gauge};
use utils::{format_size,lookup};

enum Selection {
    Category(usize),
    Path(usize, usize),
}

#[derive(Parser)]
struct Cli {
    path: Option<PathBuf>,
}




fn to_flat_index(selection: &Selection) -> usize {
    match selection {
        Selection::Category(i) => *i,
        Selection::Path(i, j) => i + 1 + j,
    }
}

fn build_list_items<'a>(
    file_map: &'a HashMap<String, Vec<PathBuf>>,
    category_size: &'a HashMap<String, f32>,
    file_size: &'a HashMap<PathBuf, u64>,
    selection: &Selection,
    is_expanded: bool,
    to_delete: &Vec<PathBuf>,
) -> Vec<ListItem<'a>> {
    let mut items: Vec<ListItem> = Vec::new();
    let mut keys: Vec<&String> = file_map.keys().collect();
    keys.sort();

    let expanded_category = match selection {
        Selection::Category(i) => *i,
        Selection::Path(i, _) => *i,
    };

    for (i, key) in keys.iter().enumerate() {
        let size = category_size.get(*key).unwrap_or(&0.0);
        let is_selected = expanded_category == i;
        let arrow = if is_expanded && is_selected { "▼" } else { "▶" };

        items.push(ListItem::new(format!(
            " {} {:<20} {}",
            arrow, key, format_size(*size)
        )));

        if is_expanded && is_selected {
            for path in &file_map[*key] {
                let path_size = *file_size.get(path).unwrap_or(&0) as f32;

                let item_style = if to_delete.contains(path) {
                    Style::default().fg(Color::Red)
                } else {
                    Style::default().fg(Color::Green)
                };

                items.push(
                    ListItem::new(format!(
                        "      {} {}",
                        format_size(path_size),
                        path.display()
                    ))
                        .style(item_style),
                );
            }
        }
    }

    items
}


fn render(
    frame: &mut Frame,
    file_map: &HashMap<String, Vec<PathBuf>>,
    category_size: &HashMap<String, f32>,
    file_size: &HashMap<PathBuf, u64>,
    list_state: &mut ListState,
    is_expanded: bool,
    selection: &Selection,
    to_delete: &Vec<PathBuf>,
    to_delete_size:f32,
    confirm:bool,
    confirm_choice:bool,
    deleted:i32
) {
    let bg = Block::default().style(Style::default().bg(Color::Rgb(22, 27, 51)));

    let [outer_layer] = Layout::vertical([Fill(1)]).areas(frame.area());

    let instructions = if confirm {
        Line::from(vec![
            " Navigate ".into(),
            "<←/→>".blue().bold(),
            " Confirm ".into(),
            "<ENTER>".blue().bold(),
            " Cancel ".into(),
            "<d>".blue().bold(),
        ])
    } else {
        Line::from(vec![
            " Move ".into(),
            "<↑/↓>".blue().bold(),
            " SubMenu ".into(),
            "<←/→>".blue().bold(),
            " Select ".into(),
            "<SPACE>".blue().bold(),
            " Delete ".into(),
            "<d>".blue().bold(),
            " Quit ".into(),
            "<q>".blue().bold(),
        ])
    };
    let outside_box = Block::default()
        .title("Rusty Cleaner v0.2.1")
        .title_style(Style::default().fg(Color::Rgb(158, 42, 43)))
        .title_bottom(instructions.centered());

    let inner_area = outside_box.inner(outer_layer);
    let [top_layer, selection_layer, confirm_layer] = Layout::vertical([Length(3), Fill(10), Length(5)]).areas(inner_area);

    let top_box = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Rgb(51, 92, 103)));

    let selection_box = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Rgb(224, 159, 62)));

    let confirm_box = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Rgb(51, 92, 103)));

    let top_text = Paragraph::new(format!(
        " TOTAL 'WASTED' SPACE : {}",
        format_size(*category_size.get("total").unwrap_or(&0.0))
    ))
        .style(Style::default().fg(Color::Rgb(229, 229, 229)).add_modifier(Modifier::BOLD))
        .block(top_box);

    let items = build_list_items(file_map, category_size, file_size, selection, is_expanded, to_delete);

    let list = List::new(items)
        .block(selection_box)
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        .highlight_symbol(">> ");


    let free_space = Text::from(vec![
        Line::from(format!("TOTAL SPACE TO RECLAIM : {}", format_size(to_delete_size))),
        Line::from(format!("{} FOLDERS SELECTED", to_delete.len()))
    ]
    );

    let free_space_text = Paragraph::new(free_space)
        .style(Style::default().fg(Color::Rgb(229, 229, 229)).add_modifier(Modifier::BOLD))
        .block(confirm_box);

    frame.render_widget(bg, frame.area());
    frame.render_widget(outside_box, outer_layer);


    frame.render_widget(top_text, top_layer);
    frame.render_stateful_widget(list, selection_layer, list_state);
    frame.render_widget(free_space_text,confirm_layer);

    if confirm{
        let centered_area = inner_area.centered(Constraint::Length(50), Constraint::Length(10));
        frame.render_widget(Clear, centered_area);

        let popup_block = Block::bordered().title("Confirmation").bg(Color::Rgb(22, 27, 51));

        let [confirm_text_area,loading, confirm_select_area] = Layout::vertical([Fill(2),Fill(1),Fill(2)]).areas(centered_area.inner(Margin::new(1,1)));


        let confirm_paragraph = Text::from(vec![
            Line::from("DELETE THE SELECTED DIRECTORIES ?"),
            Line::from(format!("{} FOLDERS SELECTED / {} TO FREE", to_delete.len(), format_size(to_delete_size))),
        ]
        ).centered();

        let [yes_selection, no_selection] = Layout::horizontal([Fill(1), Fill(1)]).areas(confirm_select_area);


        let (yes_style, no_style) = if confirm_choice {
            (
                Style::default().fg(Color::White).bg(Color::Green).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )
        } else {
            (
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                Style::default().fg(Color::White).bg(Color::Red).add_modifier(Modifier::BOLD),
            )
        };

        let yes_button = Paragraph::new("YES")
            .centered()
            .style(yes_style)
            .block(Block::bordered());

        let no_button = Paragraph::new("NO")
            .centered()
            .style(no_style)
            .block(Block::bordered());

        if deleted >= 0{
            let loading_bar = Gauge::default()
                .block(Block::bordered().title(format!(
                    "Deleting... {}/{}", deleted, to_delete.len()
                )))
                .gauge_style(Style::new().white().on_black().italic())
                .percent(if to_delete.is_empty() {
                    0
                } else {
                    (deleted * 100 / to_delete.len() as i32) as u16
                });
            frame.render_widget(loading_bar, loading);
        }

        frame.render_widget(popup_block, centered_area);
        frame.render_widget(confirm_paragraph, confirm_text_area);
        frame.render_widget(yes_button,yes_selection);
        frame.render_widget(no_button,no_selection);

    }
}

fn run_app(
    terminal: &mut ratatui::DefaultTerminal,
    file_map: &HashMap<String, Vec<PathBuf>>,
    category_size: &HashMap<String, f32>,
    file_size: &HashMap<PathBuf, u64>,
    to_delete: &mut Vec<PathBuf>,
) -> std::io::Result<()> {
    let mut list_state = ListState::default();
    let mut is_expanded = false;
    let mut selection = Selection::Category(0);

    let mut confirm = false;

    let mut confirm_choice = false;

    let mut deleted= -1; // Set to minus one, so I don't need to pass another var to render to activate it. Simply pass it to 0

    loop {

        let to_delete_size: f32 = to_delete
            .iter()
            .map(|p| *file_size.get(p).unwrap_or(&0) as f32)
            .sum();

        let mut keys: Vec<&String> = file_map.keys().collect();
        keys.sort();

        let flat = to_flat_index(&selection);
        list_state.select(Some(flat));

        terminal.draw(|frame| render(
            frame,
            file_map,
            category_size,
            file_size,
            &mut list_state,
            is_expanded,
            &selection,
            to_delete,
            to_delete_size,
            confirm,
            confirm_choice,
            deleted
        ))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => return Ok(()),

                    KeyCode::Right => {
                        if !confirm {
                            is_expanded = true
                        }
                        else { confirm_choice = false }
                    }

                    KeyCode::Left => {
                        if !confirm {
                            match &selection {
                                Selection::Path(i, _) => selection = Selection::Category(*i),
                                Selection::Category(_) => is_expanded = false,
                            }
                        }
                        else { confirm_choice = true }
                    }

                    KeyCode::Down => {
                        if !confirm {
                            match &selection {
                                Selection::Category(i) => {
                                    if is_expanded {
                                        selection = Selection::Path(*i, 0);
                                    } else {
                                        selection = Selection::Category((i + 1).min(keys.len() - 1));
                                    }
                                }
                                Selection::Path(i, j) => {
                                    let paths_len = file_map[keys[*i]].len();
                                    if j + 1 < paths_len {
                                        selection = Selection::Path(*i, j + 1);
                                    } else {
                                        selection = Selection::Category((i + 1).min(keys.len() - 1));
                                        is_expanded = false;
                                    }
                                }
                            }
                        }
                    }

                    KeyCode::Up => {
                        if !confirm {
                            match &selection {
                                Selection::Category(i) => {
                                    selection = Selection::Category(i.saturating_sub(1));
                                }
                                Selection::Path(i, j) => {
                                    if *j == 0 {
                                        selection = Selection::Category(*i);
                                    } else {
                                        selection = Selection::Path(*i, j - 1);
                                    }
                                }
                            }
                        }
                    }

                    KeyCode::Tab => {
                        if !confirm {
                            match &selection {
                                Selection::Category(i) => {
                                    selection = Selection::Category((i + 1) % keys.len());
                                }
                                _ => {}
                            }
                        }
                    }

                    KeyCode::Char(' ') => {
                        if !confirm {
                            match &selection {
                                Selection::Path(i, j) => {
                                    let mut keys: Vec<&String> = file_map.keys().collect();
                                    keys.sort();
                                    let path = file_map[keys[*i]][*j].clone();

                                    if to_delete.contains(&path) {
                                        to_delete.retain(|p| p != &path);
                                    } else {
                                        to_delete.push(path);
                                    }
                                }
                                Selection::Category(_) => {}
                            }
                        }
                    }

                    KeyCode::Enter => if confirm {
                        if confirm_choice {
                            deleted = 0;

                            let (del_tx, del_rx) = mpsc::channel::<u8>();

                            let paths = to_delete.clone();

                            let _ = thread::spawn(move || {
                                for path in paths{
                                    fs::remove_dir_all(path).unwrap();
                                    del_tx.send(1).unwrap();
                                }
                            });



                            while deleted < to_delete.len() as i32 {

                                while let Ok(_) = del_rx.try_recv() {
                                    deleted += 1;
                                }

                                terminal.draw(|frame| render(
                                    frame,
                                    file_map,
                                    category_size,
                                    file_size,
                                    &mut list_state,
                                    is_expanded,
                                    &selection,
                                    to_delete,
                                    to_delete_size,
                                    confirm,
                                    confirm_choice,
                                    deleted
                                ))?;

                                thread::sleep(Duration::from_millis(16));
                            }

                            return Ok(())
                        }
                        else {
                            confirm = false
                        }

                    }

                    else {
                        confirm = false
                    }

                    KeyCode::Char('d') => {
                        confirm = !confirm;
                    }

                    _ => {}
                }
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let args = Cli::parse();

    let start_path = args.path.clone().unwrap_or_else(|| PathBuf::from("."));

    let handle = thread::spawn(move || {
        let mut file_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        lookup(&mut file_map, &start_path);
        file_map
    });

    let spinner = ["|", "/", "-", "\\"];
    let mut i = 0;

    while !handle.is_finished() {
        print!("\rScanning {} ", spinner[i % spinner.len()]);
        std::io::Write::flush(&mut std::io::stdout())?;
        i += 1;
        thread::sleep(Duration::from_millis(100));
    }
    println!("\rScan completed!   ");

    let file_map = handle.join().unwrap();
    let all_paths: Vec<PathBuf> = file_map.values().flatten().cloned().collect();

    let size_handle = thread::spawn(move || {
        let mut file_size: HashMap<PathBuf, u64> = HashMap::new();
        for path in all_paths {
            let dir_size = get_size(&path).unwrap_or(0);
            file_size.insert(path, dir_size);
        }
        file_size
    });

    i = 0;

    while !size_handle.is_finished() {
        print!("\rCalculating size {} ", spinner[i % spinner.len()]);
        std::io::Write::flush(&mut std::io::stdout())?;
        i += 1;
        thread::sleep(Duration::from_millis(100));
    }
    println!("\rCalculating size completed!   ");

    let file_size = size_handle.join().unwrap();

    let mut global_size: f32 = 0.0;
    let mut category_size: HashMap<String, f32> = HashMap::new();

    for (key, paths) in &file_map {
        let mut total_size: f32 = 0.0;
        for path in paths {
            let size = *file_size.get(path).unwrap_or(&0) as f32;
            total_size += size;
        }
        category_size.insert(key.to_string(), total_size);
        global_size += total_size;
    }

    category_size.insert("total".to_string(), global_size);

    let mut to_delete: Vec<PathBuf> = Vec::new();

    let mut terminal = ratatui::init();
    let result = run_app(&mut terminal, &file_map, &category_size, &file_size, &mut to_delete);
    ratatui::restore();

    result
}