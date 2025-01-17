# Helium Config Service CLI

Cli tool to interact with [Helium Config Service](https://github.com/helium/oracles/tree/main/iot_config).

## Installation

Download the latest binary for your platform here from
[Releases](https://github.com/helium/helium-config-service-cli/releases/latest). Unpack
the zip file and place the `helium-config-service-cli` binary in your `$PATH`
somewhere.

### Org sol commands

Commands that interact direclty with the block chain dont require `HELIUM_CONFIG_HOST` or `HELIUM_CONFIG_PUBKEY` to be executed. However all read/writes of the db commands do so its convient to just have them included.

```
❯ helium-config-service-cli org create-helium --owner <owner pubkey> --net-id <net id type> --solana-url <solana rpc url> --solana-keypair <solanawallet.json> --commit
== Helium Organization Created: 2242 ==
== Call `org get --oui 2242 to see its details` ==
```

```
❯ helium-config-service-cli org approve --oui 2242 --solana-url <solana rpc url> --solana-keypair <solanawallet.json> --commit
== Organization Approved: 2242 ==
```

```
❯ HELIUM_CONFIG_HOST=<config host> HELIUM_CONFIG_PUBKEY=<config pubkey> HELIUM_KEYPAIR_BIN=<keypair bin> helium-config-service-cli org enable --oui 2242 --commit
OUI 2242 enabled
```

```
❯ helium-config-service-cli org update devaddr-constraint-add --oui 2245 --num-blocks 3 --solana-url <solana rpc url> --solana-keypair <solanawallet.json> --commit
== Organization Updated ==
== Call `org get --oui 2242 to see its details ==

❯ HELIUM_CONFIG_HOST=<config host> HELIUM_CONFIG_PUBKEY=<config pubkey> HELIUM_KEYPAIR_BIN=<keypair bin> helium-config-service-cli org get --oui 2242
{
  "org": {
    "oui": 2242,
    "address": <solana address>,
    "owner": <owner pubkey>,
    "escrow_key": "OUI_2242",
    "delegate_keys": [],
    "approved": true,
    "locked": false
  },
  "net_id": "00003C",
  "devaddr_constraints": [
    {
      "start_addr": "780001B9",
      "end_addr": "780001D1"
    }
  ]
}
```
