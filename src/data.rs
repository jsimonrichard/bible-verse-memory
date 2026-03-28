use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewGrade {
    Good,
    Hard,
    Again,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryVerse {
    pub reference: String,
    pub text: String,
    pub added_date: NaiveDate,
    /// Spaced repetition level (0 = brand new, higher = more mastered)
    pub level: u32,
    pub last_reviewed: Option<NaiveDate>,
    pub review_count: u32,
    /// Explicitly scheduled next review date (load-balanced across days).
    /// `None` means the verse is due immediately (new or legacy data).
    #[serde(default)]
    pub next_review: Option<NaiveDate>,
}

impl MemoryVerse {
    pub fn new(reference: String, text: String) -> Self {
        Self {
            reference,
            text,
            added_date: chrono::Utc::now().date_naive(),
            level: 0,
            last_reviewed: None,
            review_count: 0,
            next_review: None,
        }
    }

    /// Review interval in days based on the current level.
    /// Level 0: 1 day, Level 1: 2 days, Level 2: 4 days, Level 3: 7 days,
    /// Level 4: 14 days, Level 5: 30 days, Level 6: 60 days, Level 7+: 90 days
    pub fn interval_days(&self) -> i64 {
        match self.level {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 7,
            4 => 14,
            5 => 30,
            6 => 60,
            _ => 90,
        }
    }

    pub fn is_due(&self, today: NaiveDate) -> bool {
        match self.next_review {
            Some(nr) => today >= nr,
            None => match self.last_reviewed {
                None => true,
                Some(last) => (today - last).num_days() >= self.interval_days(),
            },
        }
    }

    /// Days until next review (negative means overdue)
    pub fn days_until_due(&self, today: NaiveDate) -> i64 {
        match self.next_review {
            Some(nr) => (nr - today).num_days(),
            None => match self.last_reviewed {
                None => 0,
                Some(last) => self.interval_days() - (today - last).num_days(),
            },
        }
    }

    fn mark_good(&mut self, today: NaiveDate) {
        self.level = self.level.saturating_add(1).min(8);
        self.last_reviewed = Some(today);
        self.review_count += 1;
    }

    /// Couldn't fully recall, but read/studied it — same interval repeats
    fn mark_hard(&mut self, today: NaiveDate) {
        self.last_reviewed = Some(today);
        self.review_count += 1;
    }

    /// Really struggling — drop one level (not reset to 0)
    fn mark_again(&mut self, today: NaiveDate) {
        self.level = self.level.saturating_sub(1);
        self.last_reviewed = Some(today);
        self.review_count += 1;
    }

    pub fn status_label(&self, today: NaiveDate) -> &'static str {
        if self.last_reviewed.is_none() {
            return "New";
        }
        let days = self.days_until_due(today);
        if days <= 0 {
            "Due"
        } else {
            match self.level {
                0..=1 => "Learning",
                2..=4 => "Reviewing",
                _ => "Mastered",
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VerseCollection {
    pub verses: Vec<MemoryVerse>,
}

impl VerseCollection {
    pub fn new() -> Self {
        Self { verses: Vec::new() }
    }

    pub fn add(&mut self, verse: MemoryVerse) {
        self.verses.push(verse);
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.verses.len() {
            self.verses.remove(index);
        }
    }

    pub fn due_verses(&self, today: NaiveDate) -> Vec<(usize, &MemoryVerse)> {
        self.verses
            .iter()
            .enumerate()
            .filter(|(_, v)| v.is_due(today))
            .collect()
    }

    pub fn due_count(&self, today: NaiveDate) -> usize {
        self.verses.iter().filter(|v| v.is_due(today)).count()
    }

    /// Fill in `next_review` for legacy data that only has `last_reviewed`.
    pub fn migrate(&mut self) {
        for verse in &mut self.verses {
            if verse.next_review.is_none()
                && let Some(last) = verse.last_reviewed
            {
                verse.next_review = Some(last + Duration::days(verse.interval_days()));
            }
        }
    }

    /// Grade a verse and schedule its next review with load balancing.
    pub fn mark_and_schedule(&mut self, index: usize, grade: ReviewGrade, today: NaiveDate) {
        match grade {
            ReviewGrade::Good => self.verses[index].mark_good(today),
            ReviewGrade::Hard => self.verses[index].mark_hard(today),
            ReviewGrade::Again => self.verses[index].mark_again(today),
        }
        self.schedule_next_review(index, today);
    }

    /// Pick the least-loaded day within a fuzz window for the verse's next review.
    fn schedule_next_review(&mut self, index: usize, today: NaiveDate) {
        let interval = self.verses[index].interval_days();
        let base = today + Duration::days(interval);
        let fuzz = Self::fuzz_days(interval);

        let mut best_day = base;
        let mut best_count = usize::MAX;
        for offset in 0..=fuzz {
            let candidate = base + Duration::days(offset);
            let count = self.count_scheduled_on(candidate, index);
            if count < best_count {
                best_count = count;
                best_day = candidate;
            }
        }

        self.verses[index].next_review = Some(best_day);
    }

    fn count_scheduled_on(&self, date: NaiveDate, exclude_index: usize) -> usize {
        self.verses
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != exclude_index)
            .filter(|(_, v)| v.next_review == Some(date))
            .count()
    }

    fn fuzz_days(interval: i64) -> i64 {
        match interval {
            0..=1 => 0,
            2 => 1,
            3..=4 => 2,
            5..=7 => 3,
            8..=14 => 4,
            15..=30 => 7,
            31..=60 => 10,
            _ => 14,
        }
    }
}
