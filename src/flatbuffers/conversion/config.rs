use std::collections::HashMap;

use crate::{
    flatbuffers::{
        conversion::{FromFlatbuffers, ToFlatbuffers, error::ConversionError},
        tcrm_task_generated,
    },
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

        let args = fb_config.args().map(|args_vec| {
            args_vec
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        });

        let env = fb_config.env().map(|vec| {
            vec.iter()
                .map(|entry| (entry.key().to_string(), entry.value().to_string()))
                .collect::<HashMap<_, _>>()
        });
        let ready_indicator = fb_config
            .ready_indicator()
            .map(std::string::ToString::to_string);
        let ready_indicator_source = fb_config.ready_indicator_source().try_into().ok();

        Ok(TaskConfig {
            command,
            args,
            working_dir: fb_config
                .working_dir()
                .map(std::string::ToString::to_string),
            env,
            timeout_ms: if fb_config.timeout_ms() == 0 {
                None
            } else {
                Some(fb_config.timeout_ms())
            },
            enable_stdin: Some(fb_config.enable_stdin()),
            ready_indicator,
            ready_indicator_source,
            use_process_group: Some(fb_config.use_process_group()),
        })
    }
}

impl FromFlatbuffers<tcrm_task_generated::tcrm::task::TaskConfig<'_>> for TaskConfig {
    fn from_flatbuffers(
        fb_config: tcrm_task_generated::tcrm::task::TaskConfig<'_>,
    ) -> Result<Self, ConversionError> {
        let command = fb_config.command().to_string();

        let args = fb_config.args().map(|args_vec| {
            args_vec
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        });

        let working_dir = fb_config
            .working_dir()
            .map(std::string::ToString::to_string);

        let env = fb_config.env().map(|env_vec| {
            env_vec
                .iter()
                .map(|entry| {
                    let key = entry.key().to_string();
                    let value = entry.value().to_string();
                    (key, value)
                })
                .collect()
        });

        let timeout_ms = if fb_config.timeout_ms() == 0 {
            None
        } else {
            Some(fb_config.timeout_ms())
        };
        let enable_stdin = if fb_config.enable_stdin() {
            Some(true)
        } else {
            None
        };
        let ready_indicator = fb_config
            .ready_indicator()
            .map(std::string::ToString::to_string);
        let ready_indicator_source =
            Some(StreamSource::try_from(fb_config.ready_indicator_source())?);

        let use_process_group = if fb_config.use_process_group() {
            Some(true)
        } else {
            None
        };
        Ok(TaskConfig {
            command,
            args,
            working_dir,
            env,
            timeout_ms,
            enable_stdin,
            ready_indicator,
            ready_indicator_source,
            use_process_group,
        })
    }
}

impl<'a> ToFlatbuffers<'a> for TaskConfig {
    type Output = flatbuffers::WIPOffset<tcrm_task_generated::tcrm::task::TaskConfig<'a>>;

    fn to_flatbuffers(&self, builder: &mut flatbuffers::FlatBufferBuilder<'a>) -> Self::Output {
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

        let ready_indicator_offset = self
            .ready_indicator
            .as_ref()
            .map(|s| builder.create_string(s));

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
                ready_indicator: ready_indicator_offset,
                ready_indicator_source: self
                    .ready_indicator_source
                    .clone()
                    .unwrap_or_default()
                    .into(),
                use_process_group: self.use_process_group.unwrap_or_default(),
            },
        )
    }
}
