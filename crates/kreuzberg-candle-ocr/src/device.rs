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
                if cfg!(feature = "cuda") {
                    Device::new_cuda(0).unwrap_or(Device::Cpu)
                } else if cfg!(feature = "metal") {
                    Device::new_metal(0).unwrap_or(Device::Cpu)
                } else {
                    Device::Cpu
                }
            }
        };
        Ok(device)
    }
}
