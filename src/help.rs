//! Help text (PLAN.md "CLI and scripting" / Help): `magvault --help` lists commands only,
//! grouped, one line each; `magvault <command> --help` and `magvault help <command>` show
//! one command: purpose, flags, one example.

pub const TOP: &str = "\
magvault — archive and retrieval for camera media mags

USAGE
  magvault [--config <dir>] <command> [options]
  magvault <command> --help      purpose, flags and an example for one command
  magvault help <command>        the same

SERVER COMMANDS (config without `mode: client`)
  init        create a config directory, data roots, starter profiles and the database
  ingest      verified copy of a camera card into mags/, with magcard, ASC MHL and database
  reconcile   compare magcards with the database (database wins for metadata)
  serve       commit mags uploaded by clients
  gate        forced command for a client's SSH key (not run by hand)

CLIENT COMMANDS (config with `mode: client`)
  init        create a client config (init --mode client)
  ingest      card to local staging, field set or server; sent to the server
  sync        send pending mags to the server and mark them synced
  usb         field backup sets: status, prepare, clean
  projects    open projects (from the server, else the local cache)

BOTH
  cards       list detected camera cards

GLOBAL
  --config <dir>   config directory (else MAGVAULT_CONFIG, else ~/.config/magvault/config-dir,
                   else /etc/magvault/config-dir)
  --version        print the version
";

const INIT: &str = "\
magvault init — create a config directory

Creates path.conf, the data roots, starter profiles and (server) the database. Running it
again on an existing config only fills in what is missing.

FLAGS
  --root <dir>        data directory for a new path.conf (required the first time)
  --mode client       create a client config (local staging, logs, server link) instead
  --server <host>     client: server host written to path.conf
  --write-pointer     remember the config dir in ~/.config/magvault/config-dir

EXAMPLE
  magvault --config /srv/magvault/config init --root /srv/magvault --write-pointer
";

const INGEST: &str = "\
magvault ingest — verified copy of a camera card

Probes the card (the path given, or the one detected card), shows the guesses, asks before
copying, hashes every file (SHA-256) and verifies the copy. Server: into mags/ with
magcard.txt, ASC MHL, database rows and a queued 320p job. Client: to local staging (or the
field set), then to the server, which verifies again and commits.

USAGE
  magvault ingest [<card path>] --project <name> [options]

FLAGS
  --project <name>       open project with this name, or a new one
  --project-id <uuid>    an existing open project
  --camera <id>          camera profile (default: guessed from the card)
  --format <v>           file format from the camera profile (default: profile default)
  --gamma <v>            gamma from the camera profile
  --gamut <v>            gamut from the camera profile
  --shoot-date <date>    YYYY-MM-DD (default: earliest media date)
  --notes <text>         mag-level notes
  --yes, -y              accept the guesses and the single detected card; no prompts
  --scan-root <dir>      look for cards in the folders of <dir> instead of mounts (repeatable)

CLIENT ONLY
  --via staging|set|direct   staging (default): card → local staging → server;
                             set: card → field set master → slave → server;
                             direct: card → server, no local copy
  --mirror-set               also copy to the field backup set
  --set <label>              which field set, if several are mounted
  --no-sync                  do not contact the server now; the mag stays pending

EXAMPLE
  magvault ingest /media/dit/A7SIII_128 --project ACME_Campaign_2025
";

const RECONCILE: &str = "\
magvault reconcile — compare magcards with the database

The database wins for metadata: a differing magcard is backed up and rewritten. Manifest,
file and orphan-folder differences are reported (exit code 1), never repaired.

USAGE
  magvault reconcile <mag folder>... | --all [--verify-hashes]

FLAGS
  --all              every mag, plus folders in mags/ that have no database record
  --verify-hashes    re-hash files against the stored SHA-256

EXAMPLE
  magvault reconcile --all --verify-hashes
";

const SERVE: &str = "\
magvault serve — commit mags uploaded by clients

Watches the client spool (mags/.incoming/.clients), verifies each upload against the
SHA-256 in its magcard, then commits it like a server ingest. Publishes the open-project
list for clients. Runs until Ctrl-C.

FLAGS
  --once    one pass, then exit

EXAMPLE
  magvault --config /srv/magvault/config serve
";

const GATE: &str = "\
magvault gate — forced command for a client's SSH key

Reads SSH_ORIGINAL_COMMAND and allows only: upload into this client's spool (rrsync -wo),
commit <folder>, status <folder>, projects, ping. Everything else is refused. Put it in
the client's authorized_keys line; it is not run by hand.

FLAGS
  --spool <dir>      the server's client spool (mags/.incoming/.clients)
  --results <dir>    the server's results folder (mags/.incoming/.results)
  --client <label>   this client's name (letters, digits, '-')
  --rrsync <path>    rrsync to run for uploads (default /usr/bin/rrsync)

EXAMPLE
  restrict,command=\"/usr/local/bin/magvault gate --spool /srv/magvault/mags/.incoming/.clients --results /srv/magvault/mags/.incoming/.results --client dit-a\" ssh-ed25519 AAAA… dit-a
";

const CARDS: &str = "\
magvault cards — list detected camera cards

Checks mounted filesystems under /media, /run/media and /mnt for camera layouts and shows
each card with its layout, guessed camera, file count and size. Field backup drives and
mag folders are never listed.

FLAGS
  --scan-root <dir>   treat each folder in <dir> as a mounted volume (repeatable)

EXAMPLE
  magvault cards
";

const SYNC: &str = "\
magvault sync — send pending mags to the server (client)

Re-verifies each pending mag (local staging and mounted field sets) against its stored
SHA-256, uploads it, waits for the server to commit, then removes it from staging or marks
it synced on both drives of the set. Exit code 1 if anything failed or a set needs both
drives mounted.

FLAGS
  --set <label>       only this field set
  --scan-root <dir>   field-set drives as folders in <dir> (tests; repeatable)

EXAMPLE
  magvault sync
";

const USB: &str = "\
magvault usb — field backup sets (client)

A set is one master and one slave USB drive. Prepare formats the whole drive as ext4
(sudo); Clean deletes the mags of a fully synced set and keeps the set.

USAGE
  magvault usb status
  magvault usb prepare                                      interactive: master, then slave
  magvault usb prepare-master --device <dev> --label <SET> [--yes]
  magvault usb prepare-slave  --device <dev> [--set <SET>] [--yes]
  magvault usb clean [--set <SET>] [--yes]

FLAGS
  --device <dev>      whole USB disk, e.g. /dev/sdb (never an SD card slot or camera card)
  --label <SET>       set label: A-Z, 0-9, '-', up to 12 characters
  --set <SET>         which set (if several are mounted)
  --yes, -y           skip the confirmation (sudo still asks for the password)
  --scan-root <dir>   drives as folders in <dir> (tests; repeatable)

Rules: mount the smaller drive first and prepare it as master; a smaller slave is refused,
a larger one is limited to the master's size; Prepare is blocked while mags are pending;
Clean only when every mag is synced and both drives are mounted.

EXAMPLE
  magvault usb prepare-master --device /dev/sdb --label FIELD-A
";

const PROJECTS: &str = "\
magvault projects — open projects (client)

Lists the server's open projects (UUID and name); when the server cannot be reached, the
cached list plus projects this client created itself.

EXAMPLE
  magvault projects
";

/// Help for one command, `None` for an unknown command.
pub fn command(name: &str) -> Option<&'static str> {
    Some(match name {
        "init" => INIT,
        "ingest" => INGEST,
        "reconcile" => RECONCILE,
        "serve" => SERVE,
        "gate" => GATE,
        "cards" => CARDS,
        "sync" => SYNC,
        "usb" => USB,
        "projects" => PROJECTS,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_lists_commands_without_flags() {
        for c in ["init", "ingest", "reconcile", "serve", "gate", "cards", "sync", "usb", "projects"] {
            assert!(TOP.lines().any(|l| l.trim_start().starts_with(&format!("{c} "))), "{c} missing");
            assert!(command(c).is_some(), "no help for {c}");
        }
        for flag in ["--project", "--camera", "--via", "--device", "--spool", "--verify-hashes", "--once"] {
            assert!(!TOP.contains(flag), "top-level help shows {flag}");
        }
    }

    #[test]
    fn command_help_has_purpose_and_example() {
        for c in ["init", "ingest", "reconcile", "serve", "gate", "cards", "sync", "usb", "projects"] {
            let h = command(c).unwrap();
            assert!(h.starts_with(&format!("magvault {c} — ")), "{c}");
            assert!(h.contains("\nEXAMPLE\n"), "{c}");
        }
        assert!(command("ingest").unwrap().contains("--camera"));
        assert!(!command("ingest").unwrap().contains("--device"));
        assert!(command("nope").is_none());
    }
}
