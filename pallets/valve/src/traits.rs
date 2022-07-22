pub trait Shutdown {
	fn is_shutdown() -> bool;
	fn shutdown();
	fn restart();
}
