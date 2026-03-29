---
bible-verse-memory: major
---

Initial release of Bible Verse Memory (`bvm`), a terminal UI for managing Bible memory verses with spaced repetition.

- **Dashboard**: View all verses with status, level, next review date, and review count
- **Add/Edit verses**: Full text input with word-level wrapping, clipboard paste support (bracketed paste), and keyboard shortcuts (Ctrl+Left/Right for word navigation, Ctrl+U to delete to line start)
- **Flashcard review**: Flip-card style flow — see the reference, try to recall, then reveal and grade
- **Three-tier grading**: Good (level up), Hard (same level, interval repeats), Again (level down by one)
- **Spaced repetition**: 8 levels with intervals from 1 to 90 days
- **Load-balanced scheduling**: Reviews are spread across days by picking the least-loaded day within a fuzz window, preventing spikes when many verses share the same interval
- **YAML storage**: Human-readable, version-control-friendly data format
- **Configurable data path**: Override via `~/.config/bible-verse-memory.yaml`
