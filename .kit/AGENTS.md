agent_rules:
  - scope: local_repo_only (Vantage)
  - forbid_external_modification: true
  - mandatory_pre_check: "vantage-verify <path>"
  - trust_level: structural_signal_first
  - action_on_cli_failure: stop_and_report
