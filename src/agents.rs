use crate::game::{Agent, AgentId, AgentKind, UNCONTROLLED_AGENT_ID};

const IMPLEMENTED_AGENTS: [Agent; 1] = [Agent {
    id: UNCONTROLLED_AGENT_ID,
    kind: AgentKind::Uncontrolled,
}];

/// Returns the agents implemented by this version of the engine.
pub fn available_agents() -> &'static [Agent] {
    &IMPLEMENTED_AGENTS
}

pub fn available_agent(id: AgentId) -> Option<Agent> {
    available_agents()
        .iter()
        .copied()
        .find(|agent| agent.id == id)
}

pub fn uncontrolled_agent() -> Agent {
    IMPLEMENTED_AGENTS[0]
}
