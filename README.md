# Bible Verse Memory

A terminal UI for managing Bible memory verses with spaced repetition scheduling. Verses are stored in a YAML file for easy reading and version control.

## Features

- **Dashboard**: View all memory verses with status, level, and next review date
- **Spaced Repetition**: Automatic review scheduling that adapts to how well you know each verse
- **Load-balanced Scheduling**: Reviews are spread across days so you don't get spikes of many verses due at once
- **Three-tier Grading**: Good (recalled), Hard (read it), Again (lost) — with gentle level adjustments
- **Flashcard Review**: Flip-card style review flow for due verses
- **YAML Storage**: All data stored in readable, diffable YAML format
- **Bracketed Paste**: Full clipboard paste support in input fields

## Usage

Install:

```bash
cargo install --path .
```

Run:

```bash
bvm
```

### Dashboard

The main screen shows all your memory verses with their current status.

- **↑/↓** or **j/k**: Navigate verses
- **Enter**: Review the selected verse
- **r**: Review all due verses
- **a**: Add a new verse
- **e**: Edit selected verse
- **d**: Delete selected verse
- **q/Esc**: Quit

### Adding a Verse

Press **a** from the dashboard to add a new verse.

- **Tab**: Switch between Reference and Text fields
- **Enter** (in Reference field): Move to Text field
- **Enter** (in Text field): Insert newline
- **Ctrl+S**: Save the verse
- **Esc**: Cancel

### Review Mode

Press **Enter** to review the selected verse, or **r** to review all due verses.

1. The verse reference is shown — try to recall the verse from memory
2. Press **Space/Enter** to reveal the verse text
3. Press **Space/Enter** again to hide the text (to re-test yourself)
4. Grade your recall:
   - **g** (Good): Recalled from memory → level increases, longer interval
   - **h** (Hard): Couldn't recall but read/studied it → same level, interval repeats
   - **a** (Again): Really struggling → level decreases by one

### Spaced Repetition

Each verse has a level (0–8) that determines how often it appears for review:

| Level | Review Interval |
|-------|----------------|
| 0     | Every day      |
| 1     | Every 2 days   |
| 2     | Every 4 days   |
| 3     | Every 7 days   |
| 4     | Every 14 days  |
| 5     | Every 30 days  |
| 6     | Every 60 days  |
| 7–8   | Every 90 days  |

Grading a verse **Good** increases its level. **Hard** keeps the same level (the interval repeats). **Again** drops the level by one — it does *not* reset to zero, so even forgotten verses won't immediately flood your daily reviews.

### Load Balancing

When a verse is graded, the system doesn't just schedule it for exactly `interval` days later — it picks the **least-loaded day** within a small window around the base due date. This prevents review spikes when many verses share the same interval and review date.

For example, if you have three verses all at level 1 (2-day interval) and review them all today, they won't all come due on the same day. Instead, the scheduler spreads them: two on one day and one on the next.

The fuzz window scales with the interval (no fuzz for daily reviews, up to ±14 days for 90-day intervals), so the scheduling stays close to the intended spacing while keeping your daily workload even.

## Data Storage

Verse data is stored (by default) in `~/.local/share/bible-verse-memory/verses.yaml` (or platform equivalent).

To change where data is stored, create a config file:

```yaml
# ~/.config/bible-verse-memory.yaml
data_path: path/to/verses.yaml
```

## Building

```bash
cargo build --release
```

The binary will be in `target/release/bvm`.

## License

Copyright (c) J. Simon Richard <jsimonrichard@gmail.com>

This project is licensed under the MIT license ([LICENSE] or <http://opensource.org/licenses/MIT>)

[LICENSE]: ./LICENSE
