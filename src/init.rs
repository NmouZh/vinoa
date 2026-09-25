//! Init orchestrator: phases 0-8 (spec §5). OWNER: lead.
use crate::cli::InitArgs;
use crate::error::Result;
use std::process::ExitCode;

pub fn run(_args: InitArgs) -> Result<ExitCode> {
    todo!("lead: wire wizard/flags -> matrix -> precheck -> plan -> write -> git -> verify -> report")
}
