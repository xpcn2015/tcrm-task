use std::collections::HashMap;

use crate::{
    flatbuffers::{conversion::error::ConversionError, tcrm_task_generated},
    tasks::config::{StreamSource, TaskConfig},
};

impl TryFrom<tcrm_task_generated::tcrm::task::StreamSource> for StreamSource {
    type Error = ConversionError;

    fn try_from(
        fb_source: tcrm_task_generated::tcrm::task::StreamSource,
    ) -> Result<Self, Self::Error> {
        match fb_source {
            tcrm_task_generated::tcrm::task::StreamSource::Stdout => Ok(StreamSource::Stdout),
            tcrm_task_generated::tcrm::task::StreamSource::Stderr => Ok(StreamSource::Stderr),
            _ => Err(ConversionError::InvalidStreamSource(fb_source.0)),
        }
    }
}
impl From<StreamSource> for tcrm_task_generated::tcrm::task::StreamSource {
    fn from(source: StreamSource) -> Self {
        match source {
            StreamSource::Stdout => tcrm_task_generated::tcrm::task::StreamSource::Stdout,
            StreamSource::Stderr => tcrm_task_generated::tcrm::task::StreamSource::Stderr,
        }
    }
}

impl<'a> TryFrom<tcrm_task_generated::tcrm::task::TaskConfig<'a>> for TaskConfig {
    type Error = ConversionError;

    fn try_from(
        fb_config: tcrm_task_generated::tcrm::task::TaskConfig<'a>,
    ) -> Result<Self, Self::Error> {
        let command = fb_config.command().to_string();

        let args = fb_config
            .args()
            .map(|args_vec| args_vec.iter().map(|s| s.to_string()).collect());

        let env = fb_config.env().map(|vec| {
            vec.iter()
                .filter_map(|entry| Some((entry.key().to_string(), entry.value().to_string())))
                .collect::<HashMap<_, _>>()
        });

        Ok(TaskConfig {
            command,
            args,
            working_dir: fb_config.working_dir().map(|s| s.to_string()),
            env,
            timeout_ms: if fb_config.timeout_ms() == 0 {
                None
            } else {
                Some(fb_config.timeout_ms())
            },
            enable_stdin: Some(fb_config.enable_stdin()),
        })
    }
}
impl TaskConfig {
    pub fn to_flatbuffer<'a>(
        &self,
        builder: &mut flatbuffers::FlatBufferBuilder<'a>,
    ) -> flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskConfig<'a>> {
        // Required
        let command_offset = builder.create_string(&self.command);

        // Optionals
        let args_vec = self.args.as_ref().map(|args| {
            let args_offsets: Vec<_> = args.iter().map(|s| builder.create_string(s)).collect();
            builder.create_vector(&args_offsets)
        });

        let working_dir_offset = self.working_dir.as_ref().map(|s| builder.create_string(s));

        let env_vec = self.env.as_ref().map(|env_map| {
            let env_offsets: Vec<_> = env_map
                .iter()
                .map(|(k, v)| {
                    let k_off = builder.create_string(k);
                    let v_off = builder.create_string(v);
                    tcrm_task_generated::tcrm::task::EnvEntry::create(
                        builder,
                        &tcrm_task_generated::tcrm::task::EnvEntryArgs {
                            key: Some(k_off),
                            value: Some(v_off),
                        },
                    )
                })
                .collect();
            builder.create_vector(&env_offsets)
        });

        // Build TaskConfig table
        tcrm_task_generated::tcrm::task::TaskConfig::create(
            builder,
            &tcrm_task_generated::tcrm::task::TaskConfigArgs {
                command: Some(command_offset),
                args: args_vec,
                working_dir: working_dir_offset,
                env: env_vec,
                timeout_ms: self.timeout_ms.unwrap_or_default(),
                enable_stdin: self.enable_stdin.unwrap_or_default(),
            },
        )
    }
}
