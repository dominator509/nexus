# ASSUMPTIONS

| Assumption | Reason | Risk if wrong | Verification | Blocks implementation |
| --- | --- | --- | --- | --- |
| Nexus is a greenfield repository | No Nexus repository was supplied | Existing code could conflict with names and graph | EP-000 runs repository inventory before creating source | yes if code exists |
| Commercial product name is Nexus | Conversation uses Nexus consistently | Trademark conflict may require rename | Legal name and trademark search before public launch | no; internal ID remains stable |
| Production deployment is not authorized | User requested blueprints, not live deployment | Accidental external side effects | `AUTO_DEPLOY_AUTHORIZED=no` in AGENTS.md and EP-043 | no |
| DeepSeek V4 Flash remains available under acceptable terms | It is the selected V1 ReflexProvider | Cost, policy, or API change | Provider probe, contract tests, and secondary provider certification | no; fallback required |
| Bifrost remains acceptable | Preferred gateway is not irrevocable | Project or license change | EP-000 source verification and ModelGateway conformance | no |
| PostgreSQL meets initial graph and memory needs | Simplest durable baseline | Graph workloads may outgrow recursive SQL | Graph benchmarks and `WorldGraphRepository` contract | no |
| Home Assistant is available on the home edge | Locked primary home abstraction | Some users have no home node | Installer can deploy Home Assistant or disable home profile | no |
| Roku Home direct local interfaces are unknown | No supported SDK contract was supplied | Some desired camera functions may be unavailable | EP-023 performs owned-device discovery and capability certification | no; unsupported features stay disabled |
| Asterisk and carriers can lawfully place intended calls | Telephony requires PSTN and jurisdiction rules | Legal or provider restrictions | Policy profile and counsel review per target market | yes for public launch of calling |
| ICTFax can meet selected fax use cases | Locked preferred sidecar | Carrier or T.38 limitations | Real fax provider certification | no; cloud fax fallback |
| openWakeWord upstream weights are not commercially shippable | Weight training licenses may be noncommercial | Redistribution violation | EP-021 trains or acquires approved wake weights and records license | yes for shipping those weights |
| Optional providers may ship disabled | Credentials and hardware vary per customer | Marketing could overclaim support | Provider certification registry controls UI labels | no |
| Full app-store distribution credentials are not yet known | No account details supplied | iOS or Android public release blocked | PREFLIGHT optional variables and EP-043 store readiness | no for signed sideload or internal testing |
| The reference VPS does not host local generative inference | Contabo 4B benchmark was too slow for the interactive path | Different hardware might be better | Installer benchmark can enable optional local providers | no |
| No regulated certification is currently claimed | Requirements were architectural, not certification scope | False compliance claims | SECURITY.md and release copy gate | no |
| Hardware lab inventory is not yet supplied | Device models vary | Provider certification cannot complete | Fill `hardware/LAB_INVENTORY.yaml` before hardware nodes | yes for full profile |
