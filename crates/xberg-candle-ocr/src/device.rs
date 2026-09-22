use candle_core::Device;

use crate::error::Result;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevicePreference {
    #[default]
    Auto,
    Cpu,
    Cuda,
    Metal,
}

impl DevicePreference {
    pub fn select(self) -> Result<Device> {
        let device = match self {
            DevicePreference::Cpu => Device::Cpu,
            DevicePreference::Cuda => Device::new_cuda(0)?,
            DevicePreference::Metal => Device::new_metal(0)?,
            DevicePreference::Auto => {
                let (device, fallback) = if cfg!(feature = "cuda") {
                    auto_fallback("CUDA", Device::new_cuda(0))
                } else if cfg!(feature = "metal") {
                    auto_fallback("Metal", Device::new_metal(0))
                } else {
                    (Device::Cpu, None)
                };
                if let Some((accelerator, error)) = fallback {
                    tracing::warn!(
                        accelerator,
                        %error,
                        "Device preference 'auto' could not open the accelerator and is running on the CPU. \
                         A vision-OCR model on the CPU is far slower and can take hours on a document that \
                         takes minutes on the accelerator. Set the backend device explicitly to fail here \
                         instead of falling back."
                    );
                }
                device
            }
        };
        Ok(device)
    }
}

/// Decide what `Auto` runs on, and keep the reason when it cannot reach the accelerator.
///
/// `Auto` still falls back to the CPU, because a CPU-only host is a valid place to run a binary
/// built with an accelerator feature. What it must not do is discard `accelerator`'s error: that
/// turns a failed GPU init into a silent run that is orders of magnitude slower, with no log line
/// naming the cause (GH#1712).
fn auto_fallback(
    accelerator: &'static str,
    device: std::result::Result<Device, candle_core::Error>,
) -> (Device, Option<(&'static str, candle_core::Error)>) {
    match device {
        Ok(device) => (device, None),
        Err(error) => (Device::Cpu, Some((accelerator, error))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_fallback_reports_the_accelerator_error_instead_of_discarding_it() {
        let (device, fallback) = auto_fallback("CUDA", Err(candle_core::Error::Msg("no CUDA device".into())));
        assert!(
            device.is_cpu(),
            "a failed accelerator init must still yield a usable CPU device"
        );
        let (accelerator, error) = fallback.expect("the error that forced the CPU fallback must be reported");
        assert_eq!(accelerator, "CUDA", "the report must name the accelerator that failed");
        assert!(
            error.to_string().contains("no CUDA device"),
            "the report must carry the underlying error, got: {error}"
        );
    }

    #[test]
    fn auto_fallback_reports_nothing_when_the_accelerator_opens() {
        let (device, fallback) = auto_fallback("CUDA", Ok(Device::Cpu));
        assert!(device.is_cpu(), "the opened device is returned verbatim");
        assert!(fallback.is_none(), "a successful init must not report a fallback");
    }

    #[test]
    fn explicit_cpu_preference_selects_the_cpu() {
        let device = DevicePreference::Cpu.select().expect("Cpu is always selectable");
        assert!(device.is_cpu(), "an explicit Cpu preference must select the CPU");
    }
}
