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
                .map(|entry| (entry.key().to_string(), entry.value().to_string()))
                .collect::<HashMap<_, _>>()
        });
        let ready_indicator = fb_config.ready_indicator().map(|s| s.to_string());
        let ready_indicator_source = fb_config.ready_indicator_source().try_into().ok();

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
            ready_indicator,
            ready_indicator_source,
        })
    }
}
impl TaskConfig {
    pub fn to_flatbuffers<'a>(
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
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_stream_source_conversions() {
        // Test FlatBuffer to Rust conversion
        assert_eq!(
            StreamSource::try_from(tcrm_task_generated::tcrm::task::StreamSource::Stdout).unwrap(),
            StreamSource::Stdout
        );
        assert_eq!(
            StreamSource::try_from(tcrm_task_generated::tcrm::task::StreamSource::Stderr).unwrap(),
            StreamSource::Stderr
        );

        // Test Rust to FlatBuffer conversion
        assert_eq!(
            tcrm_task_generated::tcrm::task::StreamSource::from(StreamSource::Stdout),
            tcrm_task_generated::tcrm::task::StreamSource::Stdout
        );
        assert_eq!(
            tcrm_task_generated::tcrm::task::StreamSource::from(StreamSource::Stderr),
            tcrm_task_generated::tcrm::task::StreamSource::Stderr
        );
    }

    #[test]
    fn test_task_config_roundtrip() {
        let mut env = HashMap::new();
        env.insert("KEY1".to_string(), "value1".to_string());
        env.insert("KEY2".to_string(), "value2".to_string());

        let original_config = TaskConfig {
            command: "test_command".to_string(),
            args: Some(vec!["arg1".to_string(), "arg2".to_string()]),
            working_dir: Some("/tmp".to_string()),
            env: Some(env),
            timeout_ms: Some(5000),
            enable_stdin: Some(true),
            ready_indicator: Some("READY".to_string()),
            ready_indicator_source: Some(StreamSource::Stderr),
        };

        // Convert to FlatBuffer
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = original_config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);

        // Get bytes and create new FlatBuffer instance
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();

        // Convert back to Rust
        let converted_config = TaskConfig::try_from(fb_config).unwrap();

        // Verify roundtrip
        assert_eq!(original_config.command, converted_config.command);
        assert_eq!(original_config.args, converted_config.args);
        assert_eq!(original_config.working_dir, converted_config.working_dir);
        assert_eq!(original_config.env, converted_config.env);
        assert_eq!(original_config.timeout_ms, converted_config.timeout_ms);
        assert_eq!(original_config.enable_stdin, converted_config.enable_stdin);
        assert_eq!(
            original_config.ready_indicator,
            converted_config.ready_indicator
        );
        assert_eq!(
            original_config.ready_indicator_source,
            converted_config.ready_indicator_source
        );
    }

    #[test]
    fn test_task_config_minimal() {
        let original_config = TaskConfig {
            command: "minimal".to_string(),
            args: None,
            working_dir: None,
            env: None,
            timeout_ms: None,
            enable_stdin: None,
            ready_indicator: None,
            ready_indicator_source: None,
        };

        // Convert to FlatBuffer and back
        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = original_config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);

        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted_config = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(original_config.command, converted_config.command);
        assert_eq!(original_config.args, converted_config.args);
        assert_eq!(original_config.working_dir, converted_config.working_dir);
        assert_eq!(original_config.env, converted_config.env);
        assert_eq!(converted_config.timeout_ms, None); // 0 converts to None
        assert_eq!(converted_config.enable_stdin, Some(false)); // default false
        assert_eq!(
            original_config.ready_indicator,
            converted_config.ready_indicator
        );
        assert_eq!(
            converted_config.ready_indicator_source,
            Some(StreamSource::Stdout)
        ); // default
    }

    #[test]
    fn test_task_config_empty_args() {
        let config = TaskConfig {
            command: "test".to_string(),
            args: Some(vec![]),
            working_dir: None,
            env: None,
            timeout_ms: None,
            enable_stdin: None,
            ready_indicator: None,
            ready_indicator_source: None,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(converted.args, Some(vec![]));
    }

    #[test]
    fn test_task_config_empty_env() {
        let config = TaskConfig {
            command: "test".to_string(),
            args: None,
            working_dir: None,
            env: Some(HashMap::new()),
            timeout_ms: None,
            enable_stdin: None,
            ready_indicator: None,
            ready_indicator_source: None,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(converted.env, Some(HashMap::new()));
    }

    #[test]
    fn test_task_config_large_timeout() {
        let config = TaskConfig {
            command: "test".to_string(),
            args: None,
            working_dir: None,
            env: None,
            timeout_ms: Some(u64::MAX),
            enable_stdin: None,
            ready_indicator: None,
            ready_indicator_source: None,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(converted.timeout_ms, Some(u64::MAX));
    }

    #[test]
    fn test_task_config_unicode_strings() {
        let config = TaskConfig {
            command: "测试命令".to_string(),
            args: Some(vec!["参数1".to_string(), "🚀".to_string()]),
            working_dir: Some("/tmp/测试".to_string()),
            env: None,
            timeout_ms: None,
            enable_stdin: None,
            ready_indicator: Some("准备就绪".to_string()),
            ready_indicator_source: None,
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(converted.command, "测试命令");
        let args = converted.args.unwrap();
        assert_eq!(args[0], "参数1");
        assert_eq!(args[1], "🚀");
        assert_eq!(converted.working_dir.unwrap(), "/tmp/测试");
        assert_eq!(converted.ready_indicator.unwrap(), "准备就绪");
    }

    #[test]
    fn test_stream_source_invalid() {
        let invalid_source = tcrm_task_generated::tcrm::task::StreamSource(99);
        let result = StreamSource::try_from(invalid_source);
        assert!(result.is_err());

        if let Err(ConversionError::InvalidStreamSource(val)) = result {
            assert_eq!(val, 99);
        } else {
            panic!("Expected InvalidStreamSource error");
        }
    }

    #[test]
    fn test_task_config_stress_test() {
        // Create a config with many environment variables
        let mut env = HashMap::new();
        for i in 0..100 {
            env.insert(format!("KEY_{}", i), format!("value_{}", i));
        }

        let mut args = Vec::new();
        for i in 0..50 {
            args.push(format!("arg_{}", i));
        }

        let config = TaskConfig {
            command: "stress_test_command".to_string(),
            args: Some(args),
            working_dir: Some("/tmp/stress_test".to_string()),
            env: Some(env.clone()),
            timeout_ms: Some(30000),
            enable_stdin: Some(true),
            ready_indicator: Some("STRESS_READY".to_string()),
            ready_indicator_source: Some(StreamSource::Stderr),
        };

        let mut builder = flatbuffers::FlatBufferBuilder::new();
        let fb_config = config.to_flatbuffers(&mut builder);
        builder.finish(fb_config, None);
        let bytes = builder.finished_data();
        let fb_config =
            flatbuffers::root::<tcrm_task_generated::tcrm::task::TaskConfig>(bytes).unwrap();
        let converted = TaskConfig::try_from(fb_config).unwrap();

        assert_eq!(converted.env.unwrap().len(), 100);
        assert_eq!(converted.args.unwrap().len(), 50);
        assert_eq!(converted.command, "stress_test_command");
    }
}
