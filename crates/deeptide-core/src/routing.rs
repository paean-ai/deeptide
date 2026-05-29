//! Cross-provider model routing: run the agent loop against a *primary*
//! backend — typically the local DeepSeek runtime served by `dsgo` — and
//! escalate to a *cloud* backend either on explicit user request or when the
//! primary fails.
//!
//! This is deliberately distinct from [`AnthropicConfig::fallback_model`],
//! which retries a *different model on the same provider* after a transient
//! overload (HTTP 503/529). Routing instead swaps the entire backend — a
//! different base URL, API key, and auth mode — so the "local-first,
//! cloud-on-demand" story the local-mode docs describe becomes a single
//! configured object the REPL can drive.
//!
//! The local DeepSeek V4 Flash build is documented as a feasibility-preview
//! quality model (see `docs/deepseek-v4-flash-q2.md`); routing is what makes it
//! genuinely useful — cheap/private turns stay local, and the user can lift a
//! hard turn to cloud V4 with one command instead of reconfiguring the client.
//!
//! [`AnthropicConfig::fallback_model`]: crate::AnthropicConfig

use crate::{AgentBackend, AgentRequest, AgentResponse};

/// When the router sends a turn to the escalation (cloud) backend instead of
/// the primary (local) one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EscalationPolicy {
    /// Escalate only when explicitly armed via [`RoutingBackend::arm`]. The
    /// primary handles every other turn. This is the default — predictable and
    /// user-driven ("this answer is wrong, retry it on cloud").
    #[default]
    Manual,
    /// Escalate automatically whenever the primary backend returns an error.
    /// Use when the local runtime is best-effort and any failure should fall
    /// through to cloud transparently. Arming still forces an escalation for
    /// the next turn regardless of this policy.
    OnError,
}

/// Which backend served the most recent turn. Lets the REPL/status line report
/// `local` vs `cloud` without the router exposing its internals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// Served by the primary (local) backend.
    Primary,
    /// Served by the escalation (cloud) backend.
    Escalated,
}

/// Wraps a primary [`AgentBackend`] and an optional escalation backend, routing
/// each turn to one or the other per [`EscalationPolicy`] and the one-shot
/// armed flag.
///
/// `RoutingBackend` itself implements [`AgentBackend`], so it drops into the
/// agent loop in place of a bare backend with no loop changes.
pub struct RoutingBackend {
    primary: Box<dyn AgentBackend>,
    escalation: Option<Box<dyn AgentBackend>>,
    policy: EscalationPolicy,
    armed: bool,
    last_route: Option<Route>,
}

impl RoutingBackend {
    /// Create a router that, until an escalation backend is attached, behaves
    /// exactly like `primary` alone.
    pub fn new(primary: Box<dyn AgentBackend>) -> Self {
        Self {
            primary,
            escalation: None,
            policy: EscalationPolicy::Manual,
            armed: false,
            last_route: None,
        }
    }

    /// Attach the cloud backend turns escalate to. Without this, [`arm`] is a
    /// no-op and [`EscalationPolicy::OnError`] never triggers.
    ///
    /// [`arm`]: RoutingBackend::arm
    pub fn with_escalation(mut self, escalation: Box<dyn AgentBackend>) -> Self {
        self.escalation = Some(escalation);
        self
    }

    /// Set the automatic-escalation policy. Defaults to
    /// [`EscalationPolicy::Manual`].
    pub fn with_policy(mut self, policy: EscalationPolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Route the *next* turn to the escalation backend, then auto-disarm.
    /// Returns `true` if an escalation target exists to honor the request, so
    /// the caller (e.g. a `/escalate` command) can report "no cloud configured"
    /// rather than silently pretending it armed.
    pub fn arm(&mut self) -> bool {
        if self.escalation.is_some() {
            self.armed = true;
            true
        } else {
            false
        }
    }

    /// Cancel a pending one-shot escalation.
    pub fn disarm(&mut self) {
        self.armed = false;
    }

    /// Whether the next turn is set to escalate.
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Whether a cloud escalation backend is configured.
    pub fn has_escalation(&self) -> bool {
        self.escalation.is_some()
    }

    /// Which backend served the most recent turn, or `None` before the first.
    pub fn last_route(&self) -> Option<Route> {
        self.last_route
    }
}

impl AgentBackend for RoutingBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        // An explicit one-shot escalation wins regardless of policy. Disarm
        // first so a failure can't strand the router in a permanently-armed
        // state.
        if self.armed {
            self.armed = false;
            if let Some(escalation) = self.escalation.as_mut() {
                self.last_route = Some(Route::Escalated);
                return escalation.respond(request);
            }
            // Armed with no target configured: fall through to the primary.
        }

        // OnError needs the request a second time, so clone it up front — but
        // only when a retry could actually happen, to avoid the cost otherwise.
        let can_auto_escalate =
            self.policy == EscalationPolicy::OnError && self.escalation.is_some();
        let retry_request = if can_auto_escalate {
            Some(request.clone())
        } else {
            None
        };

        match self.primary.respond(request) {
            Ok(response) => {
                self.last_route = Some(Route::Primary);
                Ok(response)
            }
            Err(primary_error) => {
                if let (Some(retry), Some(escalation)) = (retry_request, self.escalation.as_mut()) {
                    self.last_route = Some(Route::Escalated);
                    return escalation.respond(retry).map_err(|escalation_error| {
                        format!(
                            "primary backend failed ({primary_error}); \
                             escalation backend also failed ({escalation_error})"
                        )
                    });
                }
                self.last_route = Some(Route::Primary);
                Err(primary_error)
            }
        }
    }
}
