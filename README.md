# Helium Config Service CLI

Cli tool to interact with [Helium Config Service](https://github.com/helium/oracles/tree/main/iot_config).

## Installation

Download the latest binary for your platform here from
[Releases](https://github.com/helium/helium-config-service-cli/releases/latest). Unpack
the zip file and place the `helium-config-service-cli` binary in your `$PATH`
somewhere.

### Org sol commands

Commands that interact direclty with the block chain dont require `HELIUM_CONFIG_HOST` or `HELIUM_CONFIG_PUBKEY` to be executed. However all read commands do so its convient to just have them included.

```
❯ HELIUM_CONFIG_HOST=http://0.0.0.0:8080 HELIUM_CONFIG_PUBKEY=1a5qUaEAQJJufDcaxMADdDyabwXgCqgAv7C5eWZFPt4n8TiY2pV HELIUM_KEYPAIR_BIN=/Users/bry/Documents/Work/scripts/helium_keypair_gen/testnet_keypair.bin SOLANA_URL=http://127.0.0.1:8899 ./target/release/helium-config-service-cli org create-helium --owner 6om7Zt4tcCUKksK4SLuWR2Eoj5QuzACk57A5V2NB9RkV --net-id type0-0x00003c --wallet <solanawallet.json> --commit
== Helium Organization Created: 2242 ==
== Call `org get --oui 2242 to see its details` ==
```

```
❯ HELIUM_CONFIG_HOST=http://0.0.0.0:8080 HELIUM_CONFIG_PUBKEY=1a5qUaEAQJJufDcaxMADdDyabwXgCqgAv7C5eWZFPt4n8TiY2pV HELIUM_KEYPAIR_BIN=/Users/bry/Documents/Work/scripts/helium_keypair_gen/testnet_keypair.bin SOLANA_URL=http://127.0.0.1:8899 ./target/release/helium-config-service-cli org approve --oui 2242 --wallet <solanawallet.json> --commit
== Organization Approved: 2242 ==
```

```
❯ HELIUM_CONFIG_HOST=http://0.0.0.0:8080 HELIUM_CONFIG_PUBKEY=1a5qUaEAQJJufDcaxMADdDyabwXgCqgAv7C5eWZFPt4n8TiY2pV HELIUM_KEYPAIR_BIN=/Users/bry/Documents/Work/scripts/helium_keypair_gen/testnet_keypair.bin SOLANA_URL=http://127.0.0.1:8899 ./target/release/helium-config-service-cli org enable --oui 2242 --commit
OUI 2242 enabled
```

```
❯ HELIUM_CONFIG_HOST=http://0.0.0.0:8080 HELIUM_CONFIG_PUBKEY=1a5qUaEAQJJufDcaxMADdDyabwXgCqgAv7C5eWZFPt4n8TiY2pV HELIUM_KEYPAIR_BIN=/Users/bry/Documents/Work/scripts/helium_keypair_gen/testnet_keypair.bin SOLANA_URL=http://127.0.0.1:8899 ./target/release/helium-config-service-cli org update devaddr-constraint-add --oui 2242 --num-blocks 3 --wallet <solanawallet.json> --commit
== Organization Updated ==
== Call `org get --oui 2242 to see its details ==

❯ HELIUM_CONFIG_HOST=http://0.0.0.0:8080 HELIUM_CONFIG_PUBKEY=1a5qUaEAQJJufDcaxMADdDyabwXgCqgAv7C5eWZFPt4n8TiY2pV HELIUM_KEYPAIR_BIN=/Users/bry/Documents/Work/scripts/helium_keypair_gen/testnet_keypair.bin SOLANA_URL=http://127.0.0.1:8899 ./target/release/helium-config-service-cli org get --oui 2242
{
  "org": {
    "oui": 2242,
    "address": "29VuridxigUPBzK315weL7JMsVnNgvYVcy5EmDW8df3S",
    "owner": "6om7Zt4tcCUKksK4SLuWR2Eoj5QuzACk57A5V2NB9RkV",
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
