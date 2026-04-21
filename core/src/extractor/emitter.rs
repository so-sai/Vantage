use std::io::{self, ErrorKind, Write};
use vantage_types::EdgeEvent;

/// Emits an EdgeEvent as a JSONL line to the given writer.
/// Handles BrokenPipe gracefully to support CLI piping (e.g. | head).
pub fn emit_edge(edge: &EdgeEvent, writer: &mut impl Write) -> io::Result<()> {
    let json_str = serde_json::to_string(edge)
        .map_err(io::Error::other)?;
    
    if let Err(e) = writeln!(writer, "{}", json_str) {
        if e.kind() == ErrorKind::BrokenPipe {
            // Graceful shutdown if the consumer closed the pipe
            return Ok(());
        }
        return Err(e);
    }
    
    Ok(())
}
