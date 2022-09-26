/// For pallets that want to be notified on shutdown events
pub trait Shutdown {
	/// Whether or not implementer is shutdown
	fn is_shutdown() -> bool;
	/// Forwards shutdown message to implementer
	fn shutdown();
	/// Forwards restart message to implementer
	fn restart();
}

impl Shutdown for () {
	fn is_shutdown() -> bool {
		true
	}
	fn shutdown() {}
	fn restart() {}
}
