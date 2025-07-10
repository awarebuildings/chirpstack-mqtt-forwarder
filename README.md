# ChirpStack MQTT Forwarder

![CI](https://github.com/chirpstack/chirpstack-mqtt-forwarder/actions/workflows/main.yml/badge.svg?branch=master)

ChirpStack MQTT Forwarder is a Semtech UDP and ChirpStack Concentratord to
MQTT forwarder. It is intended to run on the gateway and is a more lightweight
alternative to the ChirpStack Gateway Bridge such that it can run on gateways
with a limited amount of flash memory.

## Documentation and binaries

Please refer to the [ChirpStack](https://www.chirpstack.io/) website for
documentation and pre-compiled binaries.

## Building from source

### Requirements

Building ChirpStack MQTT Forwarder requires:

* [Nix](https://nixos.org/download.html) (recommended) and
* [Docker](https://www.docker.com/)

#### Nix

Nix is used for setting up the development environment which is used for local
development and for creating the binaries.

If you do not have Nix installed and do not wish to install it, then you can
use the provided Docker Compose based Nix environment. To start this environment
execute the following command:

```bash
make docker-devshell
```

**Note:** You will be able to run the test commands and run `cargo build`, but
cross-compiling will not work within this environment (because it would try start
Docker within Docker).

#### Docker

Docker is used by [cross-rs](https://github.com/cross-rs/cross) for cross-compiling,
as well as some of the `make` commands.

### Starting the development shell

Run the following command to start the development shell:

```bash
nix-shell
```

Or if you do not have Nix installed, execute the following command:

```bash
make docker-devshell
```

### Running tests

#### Start required services

ChirpStack MQTT Forwarder depends on a MQTT broker for running the tests.
You need to start this service manually if you started the development shell
using `nix-shell`:

```bash
docker-compose up -d
```

#### Run tests

Execute the following command to run the tests:

```bash
make test
```

### Building binaries

Execute the following commands to build the ChirpStack MQTT Forwarder binaries
and packages:

```bash
# Only build binaries
make build

# Build binaries + distributable packages.
make dist
```

## Dynamic filter configuration

The ChirpStack MQTT Forwarder by default can be configured with DevAddr and
JoinEUI prefix filters for filtering uplink data.

This fork contains modifications to also support dynamic filters that can
be configured by sending JSON configuration payloads over MQTT to the
`[region]/gateway/[gateway_id]/command/filters` topic. In case a response
is requested, the response is published to `[region/gateway]/[gateway_id]/event/filters`.

### Command

The command payload format is:

```json
{
  "flush_filters": true,
  "set": {
    "0101010101010101": ["01010101", "02020202"]
  },
  "set_dev_euis": {
    "0101010101010101": ["01010101", "02020202"]
  },
  "remove_dev_euis": ["0101010101010101"],
  "return_filters": true
}
```

It is not required to provide all fields. Actions are performed in the
following order (if provided):

* `flush_filters`
* `set`
* `remove_dev_euis`
* `set_dev_euis`
* `return_filters`

#### `flush_filters`

This will flush all existing filters (before other actions are executed).

#### `set`

This will set the filterlist to exactly the filterlist provided in the
`set` command. This is effectively the same as `flush_filters` + `set_dev_euis`
with the exception that in case `set` is used, the filterlist will only be
overwritten once the full `set` payload has been parsed. In case of using
`flush_filters` + `set_dev_euis` the filters will first be flushed, after which
the `set_dev_euis` payload is decoded.

#### `remove_dev_euis`

This removes the DevEUI entries from the filterlist.

#### `set_dev_euis`

This adds (or overwrites) the given DevEUI entries to the list + the list of
DevAddrs that are associated with the DevEUI. The list of DevAddrs can be emty.

#### `return_filters`

This returns the filterlist. This can be used to get the filterlist, or can be
used combined with one of the above commands to confirm that the filterlist
has been updated.

## License

ChirpStack MQTT Forwarder is distributed under the MIT license. See also
[LICENSE](https://github.com/brocaar/chirpstack/blob/master/LICENSE).
