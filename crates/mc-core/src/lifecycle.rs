use crate::InstanceSummary;

/// A live reservation that keeps an instance stopped for an offline operation.
///
/// Implementations must retain the instance's lifecycle exclusion for the
/// lifetime of the value. Consumers can move the reservation to a blocking
/// worker while reading or replacing instance files.
pub trait StoppedInstanceLease: Send + 'static {
    fn instance(&self) -> &InstanceSummary;
}
