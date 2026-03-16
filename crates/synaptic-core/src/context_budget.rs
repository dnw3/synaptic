use crate::token_counter::TokenCounter;
use crate::Message;
use std::sync::Arc;

/// Priority level for context slots. Lower values = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(pub u8);

impl Priority {
    pub const CRITICAL: Priority = Priority(0);
    pub const HIGH: Priority = Priority(64);
    pub const NORMAL: Priority = Priority(128);
    pub const LOW: Priority = Priority(192);
}

/// Strategy for trimming a context slot when it doesn't fit the budget.
#[derive(Default)]
pub enum SlotTrimStrategy {
    /// Include all messages or none (current behavior).
    #[default]
    AllOrNone,
    /// Keep most recent messages that fit within the remaining budget.
    KeepRecent,
}

/// A slot of context to include in the budget.
pub struct ContextSlot {
    pub name: String,
    pub priority: Priority,
    pub messages: Vec<Message>,
    /// Minimum reserved tokens for this slot (guaranteed if budget allows).
    pub reserved_tokens: usize,
    /// Strategy for handling slots that exceed the remaining budget.
    pub trim_strategy: SlotTrimStrategy,
}

/// Assembles messages from multiple context slots within a token budget.
///
/// Slots are sorted by priority (lowest value = highest priority).
/// Higher-priority slots are included first. Lower-priority slots are
/// dropped if the budget is exceeded.
pub struct ContextBudget {
    max_tokens: usize,
    counter: Arc<dyn TokenCounter>,
}

impl ContextBudget {
    pub fn new(max_tokens: usize, counter: Arc<dyn TokenCounter>) -> Self {
        Self {
            max_tokens,
            counter,
        }
    }

    /// Assemble messages from slots that fit within the token budget.
    ///
    /// Slots are processed in priority order (CRITICAL first, LOW last).
    /// Each slot's messages are included if they fit. Slots with
    /// `reserved_tokens > 0` are guaranteed inclusion (if total reserved
    /// fits within budget).
    ///
    /// For slots with `SlotTrimStrategy::KeepRecent`, the most recent messages
    /// that fit within the remaining budget are kept.
    pub fn assemble(&self, mut slots: Vec<ContextSlot>) -> Vec<Message> {
        // Sort by priority (lower value = higher priority)
        slots.sort_by_key(|s| s.priority);

        let mut result = Vec::new();
        let mut used_tokens = 0;

        for slot in slots {
            let slot_tokens = self.counter.count_messages(&slot.messages);
            let remaining = self.max_tokens.saturating_sub(used_tokens);

            if slot_tokens <= remaining {
                // Fits entirely
                used_tokens += slot_tokens;
                result.extend(slot.messages);
            } else if slot.reserved_tokens > 0 && slot_tokens <= remaining {
                used_tokens += slot_tokens;
                result.extend(slot.messages);
            } else {
                match slot.trim_strategy {
                    SlotTrimStrategy::KeepRecent if remaining > 0 => {
                        // Keep the most recent messages that fit
                        let mut kept = Vec::new();
                        let mut kept_tokens = 0;
                        for msg in slot.messages.into_iter().rev() {
                            let msg_tokens = self.counter.count_text(msg.content()) + 4;
                            if kept_tokens + msg_tokens <= remaining {
                                kept_tokens += msg_tokens;
                                kept.push(msg);
                            } else {
                                break;
                            }
                        }
                        kept.reverse();
                        used_tokens += kept_tokens;
                        result.extend(kept);
                    }
                    _ => {
                        // AllOrNone or no remaining budget: skip
                    }
                }
            }
        }

        result
    }
}
