# Key module mockup (ratatui)

> **Mockup only.** Design reference for the Key module. There is no code yet.
> Character-grid drawings at the two sizes from the GUI technical spec (150x42 comfortable, 120x36 minimum). Colours, glyphs, key letters and all sample values (labels, fingerprints, drive names, the private key text) are placeholders until the shared theme is defined.

Behaviour source of truth is `PLAN.md` ("Key module"). Admin only: the module is visible only when running as the `magvault` user, and it is never shown on the client. Keys live in `~magvaultclient/.ssh/authorized_keys`. The admin is the only person who can grant access; there is no self-service enrollment.

Panes: 1 Client keys (list), 2 right pane (selected key and actions, or the add flow).

- **Generate on server:** admin enters a label (client name); the module creates the key pair, adds the public key to authorized_keys and shows the private key once. The admin then hands it over in one of two ways:
  - `w` write it to a chosen file, or
  - `u` write it directly to a mounted USB drive: the module lists removable drives, the admin picks one, and the key is written as e.g. `magvault-client.key`.
  - After either action the private key is no longer stored on the server, and a confirmation says so. Leaving the screen without writing discards it.
- **Accept a public key:** the client already generated its key pair; the admin enters a label and pastes the public key; the module adds it to authorized_keys.
- **Revoke:** removes the key from authorized_keys, after a confirm.

## Keys

| key | action |
|---|---|
| Tab / Shift+Tab | cycle panes / fields |
| Up / Down | move in the key list (or the drive list in the USB step) |
| `n` | new key (choose generate or paste) |
| `g` | generate on server |
| `p` | paste a public key |
| `x` | revoke the selected key (asks first) |
| `w` | write the generated private key to a chosen file (generate flow) |
| `u` | write the generated private key to a mounted USB drive (generate flow) |
| Enter | confirm |
| Esc | back one step; from the list, back to the dashboard |

`w` and `u` come from PLAN.md; the other letters are proposals, local to this screen.

## Key list, 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
╔═ 1 Client keys ════════════════════════════════════════════════════════════════════════╗┌─ 2 Selected key ─────────────────────────────────────────┐
║                                                                                        ║│                                                          │
║   label (client)      fingerprint                                                      ║│ label (client)  dit-cart-a                               │
║   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               ║│                                                          │
║ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               ║│ fingerprint                                              │
║   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               ║│   SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE     │
║   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               ║│                                                          │
║                                                                                        ║│                                                          │
║ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         ║│ Actions                                                  │
║                                                                                        ║│   x  Revoke this key (asks first)                        │
║                                                                                        ║│      removes it from authorized_keys                     │
║                                                                                        ║│                                                          │
║                                                                                        ║│ Add a client key                                         │
║                                                                                        ║│   n  New key   g  Generate   p  Paste                    │
║                                                                                        ║│                                                          │
║                                                                                        ║│ Only an admin (magvault) can grant access.               │
║                                                                                        ║│ No self-service enrollment.                              │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║ Public keys and labels only. Server-generated private                                  ║│                                                          │
║ keys are never kept after they have been shown.                                        ║│                                                          │
╚════════════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────────────────────────┘
 Tab pane · ↑↓ move · n new key · g generate · p paste public key · x revoke (asks) · Esc dashboard
```

## Generate on server: private key shown once, 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────────────────────┐╔═ 2 New key · generate on server ═════════════════════════╗
│                                                                                        │║                                                          ║
│   label (client)      fingerprint                                                      │║ Step 2 of 2  ·  copy the private key now                 ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               │║ label (client)  wrangler-tablet                          ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               │║ fingerprint                                              ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c     ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               │║                                                          ║
│                                                                                        │║ ✓ public key added to authorized_keys                    ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         │║                                                          ║
│                                                                                        │║ Private key (shown once)                                 ║
│ New key added: wrangler-tablet                                                         │║  -----BEGIN OPENSSH PRIVATE KEY-----                     ║
│                                                                                        │║  b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQ              ║
│                                                                                        │║  AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA             ║
│                                                                                        │║  …                                                       ║
│                                                                                        │║  -----END OPENSSH PRIVATE KEY-----                       ║
│                                                                                        │║                                                          ║
│                                                                                        │║ Hand it to the client: write it out now. It is not       ║
│                                                                                        │║ stored after that.                                       ║
│                                                                                        │║                                                          ║
│                                                                                        │║ Hand over                                                ║
│                                                                                        │║   w  Write to a chosen file                              ║
│                                                                                        │║   u  Write to a mounted USB drive                        ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│ Public keys and labels only. Server-generated private                                  │║                                                          ║
│ keys are never kept after they have been shown.                                        │║                                                          ║
└────────────────────────────────────────────────────────────────────────────────────────┘╚══════════════════════════════════════════════════════════╝
 w write to file · u write to USB drive · Esc cancel (private key is discarded)
```

## Generate: write to a chosen file (`w`), 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────────────────────┐╔═ 2 New key · write to file ══════════════════════════════╗
│                                                                                        │║                                                          ║
│   label (client)      fingerprint                                                      │║ Write the private key to a file                          ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               │║ label (client)  wrangler-tablet                          ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               │║ fingerprint                                              ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c     ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               │║                                                          ║
│                                                                                        │║ ✓ public key added to authorized_keys                    ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         │║                                                          ║
│                                                                                        │║ file                                                     ║
│ New key added: wrangler-tablet                                                         │║  /home/magvault/magvault-client.key▏                     ║
│                                                                                        │║                                                          ║
│                                                                                        │║ The private key is held in memory only until it is       ║
│                                                                                        │║ written. It is not kept on the server afterwards.        ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│ Public keys and labels only. Server-generated private                                  │║  Enter  Write    Esc  Back                               ║
│ keys are never kept after they have been shown.                                        │║                                                          ║
└────────────────────────────────────────────────────────────────────────────────────────┘╚══════════════════════════════════════════════════════════╝
 Enter write · Esc back
```

## Generate: write to a USB drive (`u`), 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────────────────────┐╔═ 2 New key · write to USB ═══════════════════════════════╗
│                                                                                        │║                                                          ║
│   label (client)      fingerprint                                                      │║ Write the private key to a USB drive                     ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               │║ label (client)  wrangler-tablet                          ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               │║ fingerprint                                              ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c     ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               │║                                                          ║
│                                                                                        │║ ✓ public key added to authorized_keys                    ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         │║                                                          ║
│                                                                                        │║ Removable drives                                         ║
│ New key added: wrangler-tablet                                                         │║ ▶ KEYSTICK      /dev/sdh1                                ║
│                                                                                        │║   BACKUP-USB    /dev/sdi1                                ║
│                                                                                        │║                                                          ║
│                                                                                        │║ file name                                                ║
│                                                                                        │║  magvault-client.key▏                                    ║
│                                                                                        │║                                                          ║
│                                                                                        │║ The private key is held in memory only until it is       ║
│                                                                                        │║ written. It is not kept on the server afterwards.        ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│ Public keys and labels only. Server-generated private                                  │║  Enter  Write    Esc  Back                               ║
│ keys are never kept after they have been shown.                                        │║                                                          ║
└────────────────────────────────────────────────────────────────────────────────────────┘╚══════════════════════════════════════════════════════════╝
 ↑↓ drive · Tab field · Enter write · Esc back
```

## Generate: written, not kept, 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────────────────────┐╔═ 2 New key · done ═══════════════════════════════════════╗
│                                                                                        │║                                                          ║
│   label (client)      fingerprint                                                      │║ Private key written                                      ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               │║ label (client)  wrangler-tablet                          ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               │║ fingerprint                                              ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c     ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               │║                                                          ║
│                                                                                        │║ ✓ public key added to authorized_keys                    ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         │║                                                          ║
│                                                                                        │║ ✓ written to KEYSTICK (/dev/sdh1)                        ║
│ New key added: wrangler-tablet                                                         │║   magvault-client.key                                    ║
│                                                                                        │║                                                          ║
│                                                                                        │║ ✓ not kept on the server                                 ║
│                                                                                        │║   The private key has been cleared from the server. Only ║
│                                                                                        │║   the public key and the label remain.                   ║
│                                                                                        │║                                                          ║
│                                                                                        │║ Safely remove the USB drive and copy the key to the      ║
│                                                                                        │║ client.                                                  ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│ Public keys and labels only. Server-generated private                                  │║  Enter  Done                                             ║
│ keys are never kept after they have been shown.                                        │║                                                          ║
└────────────────────────────────────────────────────────────────────────────────────────┘╚══════════════════════════════════════════════════════════╝
 Enter done
```

## Add key: paste a client public key, 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────────────────────┐╔═ 2 New key · paste public key ═══════════════════════════╗
│                                                                                        │║                                                          ║
│   label (client)      fingerprint                                                      │║ Step 2 of 2  ·  paste the client public key              ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               │║                                                          ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               │║ label (client name)                                      ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               │║  wrangler-tablet▏                                        ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               │║                                                          ║
│                                                                                        │║ public key                                               ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         │║  ssh-… AAAAC3NzaC1lZDI1NTE5AAAAIOk4Lw9CnT2eQk7RaZu0VbXd  ║
│                                                                                        │║  J5sMfOgP1iNtE8oYqA3c wrangler-tablet                    ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║ ✓ valid public key                                       ║
│                                                                                        │║ fingerprint                                              ║
│                                                                                        │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c     ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║ Enter adds it to ~magvaultclient/.ssh/authorized_keys.   ║
│                                                                                        │║ The client already holds its private key.                ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│                                                                                        │║                                                          ║
│ Public keys and labels only. Server-generated private                                  │║  Enter  Add key    Esc  Cancel                           ║
│ keys are never kept after they have been shown.                                        │║                                                          ║
└────────────────────────────────────────────────────────────────────────────────────────┘╚══════════════════════════════════════════════════════════╝
 Tab field · Enter add key · Esc cancel (back to list)
```

## Revoke confirm, 150x42

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                                                       running as magvault
╔═ 1 Client keys ════════════════════════════════════════════════════════════════════════╗┌─ 2 Revoke key ───────────────────────────────────────────┐
║                                                                                        ║│                                                          │
║   label (client)      fingerprint                                                      ║│ label (client)  dit-cart-a                               │
║   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV5tA               ║│ fingerprint                                              │
║ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE               ║│   SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6xDE     │
║   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw2Cu               ║│                                                          │
║   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB8tD               ║│                                                          │
║                                                                                        ║│ ┌──────────────────────────────────────────────────────┐ │
║ 4 keys in ~magvaultclient/.ssh/authorized_keys                                         ║│ │ Revoke this key?                                     │ │
║                                                                                        ║│ │  This client will no longer be                       │ │
║                                                                                        ║│ │  able to connect.                                    │ │
║                                                                                        ║│ │  It is removed from authorized_keys.                 │ │
║                                                                                        ║│ │                                                      │ │
║                                                                                        ║│ │   Enter  Revoke   Esc  Cancel                        │ │
║                                                                                        ║│ └──────────────────────────────────────────────────────┘ │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║                                                                                        ║│                                                          │
║ Public keys and labels only. Server-generated private                                  ║│                                                          │
║ keys are never kept after they have been shown.                                        ║│                                                          │
╚════════════════════════════════════════════════════════════════════════════════════════╝└──────────────────────────────────────────────────────────┘
 Enter revoke · Esc cancel
```

## Key list, 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
╔═ 1 Client keys ════════════════════════════════════════════════════════╗┌─ 2 Selected key ───────────────────────────┐
║                                                                        ║│                                            │
║   label (client)      fingerprint                                      ║│ label (client)  dit-cart-a                 │
║   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… ║│                                            │
║ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… ║│ fingerprint                                │
║   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… ║│   SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAi │
║   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… ║│   T3yQmN6xDE                               │
║                                                                        ║│                                            │
║ 4 keys in ~magvaultclient/.ssh/authorized_keys                         ║│ Actions                                    │
║                                                                        ║│   x  Revoke this key (asks first)          │
║                                                                        ║│      removes it from authorized_keys       │
║                                                                        ║│                                            │
║                                                                        ║│ Add a client key                           │
║                                                                        ║│   n  New key   g  Generate   p  Paste      │
║                                                                        ║│                                            │
║                                                                        ║│ Only an admin (magvault) can grant access. │
║                                                                        ║│ No self-service enrollment.                │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║ Public keys and labels only. Server-generated private                  ║│                                            │
║ keys are never kept after they have been shown.                        ║│                                            │
╚════════════════════════════════════════════════════════════════════════╝└────────────────────────────────────────────┘
 Tab pane · ↑↓ move · n new key · g generate · p paste public key · x revoke (asks) · Esc dashboard
```

## Generate on server: private key shown once, 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────┐╔═ 2 New key · generate ═════════════════════╗
│                                                                        │║                                            ║
│   label (client)      fingerprint                                      │║ Step 2 of 2  ·  copy the private key now   ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… │║ label (client)  wrangler-tablet            ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… │║ fingerprint                                ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1i ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… │║   NtE8oYqA3c                               ║
│                                                                        │║ ✓ public key added to authorized_keys      ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                         │║                                            ║
│                                                                        │║ Private key (shown once)                   ║
│ New key added: wrangler-tablet                                         │║  -----BEGIN OPENSSH PRIVATE KEY-----       ║
│                                                                        │║  b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9u  ║
│                                                                        │║  AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA  ║
│                                                                        │║  …                                         ║
│                                                                        │║  -----END OPENSSH PRIVATE KEY-----         ║
│                                                                        │║                                            ║
│                                                                        │║ Hand it to the client: write it out now.   ║
│                                                                        │║ It is not stored after that.               ║
│                                                                        │║                                            ║
│                                                                        │║ Hand over                                  ║
│                                                                        │║   w  Write to a chosen file                ║
│                                                                        │║   u  Write to a mounted USB drive          ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│ Public keys and labels only. Server-generated private                  │║                                            ║
│ keys are never kept after they have been shown.                        │║                                            ║
└────────────────────────────────────────────────────────────────────────┘╚════════════════════════════════════════════╝
 w write to file · u write to USB drive · Esc cancel (private key is discarded)
```

## Generate: write to a chosen file (`w`), 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────┐╔═ 2 New key · to file ══════════════════════╗
│                                                                        │║                                            ║
│   label (client)      fingerprint                                      │║ Write the private key to a file            ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… │║ label (client)  wrangler-tablet            ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… │║ fingerprint                                ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1i ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… │║   NtE8oYqA3c                               ║
│                                                                        │║ ✓ public key added to authorized_keys      ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                         │║                                            ║
│                                                                        │║ file                                       ║
│ New key added: wrangler-tablet                                         │║  /home/magvault/magvault-client.key▏       ║
│                                                                        │║                                            ║
│                                                                        │║ The private key is held in memory only     ║
│                                                                        │║ until it is written. It is not kept on the ║
│                                                                        │║ server afterwards.                         ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│ Public keys and labels only. Server-generated private                  │║  Enter  Write    Esc  Back                 ║
│ keys are never kept after they have been shown.                        │║                                            ║
└────────────────────────────────────────────────────────────────────────┘╚════════════════════════════════════════════╝
 Enter write · Esc back
```

## Generate: write to a USB drive (`u`), 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────┐╔═ 2 New key · to USB ═══════════════════════╗
│                                                                        │║                                            ║
│   label (client)      fingerprint                                      │║ Write the private key to a USB drive       ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… │║ label (client)  wrangler-tablet            ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… │║ fingerprint                                ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1i ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… │║   NtE8oYqA3c                               ║
│                                                                        │║ ✓ public key added to authorized_keys      ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                         │║                                            ║
│                                                                        │║ Removable drives                           ║
│ New key added: wrangler-tablet                                         │║ ▶ KEYSTICK      /dev/sdh1                  ║
│                                                                        │║   BACKUP-USB    /dev/sdi1                  ║
│                                                                        │║                                            ║
│                                                                        │║ file name                                  ║
│                                                                        │║  magvault-client.key▏                      ║
│                                                                        │║                                            ║
│                                                                        │║ The private key is held in memory only     ║
│                                                                        │║ until it is written. It is not kept on the ║
│                                                                        │║ server afterwards.                         ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│ Public keys and labels only. Server-generated private                  │║  Enter  Write    Esc  Back                 ║
│ keys are never kept after they have been shown.                        │║                                            ║
└────────────────────────────────────────────────────────────────────────┘╚════════════════════════════════════════════╝
 ↑↓ drive · Tab field · Enter write · Esc back
```

## Generate: written, not kept, 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────┐╔═ 2 New key · done ═════════════════════════╗
│                                                                        │║                                            ║
│   label (client)      fingerprint                                      │║ Private key written                        ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… │║ label (client)  wrangler-tablet            ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… │║ fingerprint                                ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1i ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… │║   NtE8oYqA3c                               ║
│                                                                        │║ ✓ public key added to authorized_keys      ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                         │║                                            ║
│                                                                        │║ ✓ written to KEYSTICK (/dev/sdh1)          ║
│ New key added: wrangler-tablet                                         │║   magvault-client.key                      ║
│                                                                        │║                                            ║
│                                                                        │║ ✓ not kept on the server                   ║
│                                                                        │║   The private key has been cleared from    ║
│                                                                        │║   the server. Only the public key and the  ║
│                                                                        │║   label remain.                            ║
│                                                                        │║                                            ║
│                                                                        │║ Safely remove the USB drive and copy the   ║
│                                                                        │║ key to the client.                         ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│ Public keys and labels only. Server-generated private                  │║  Enter  Done                               ║
│ keys are never kept after they have been shown.                        │║                                            ║
└────────────────────────────────────────────────────────────────────────┘╚════════════════════════════════════════════╝
 Enter done
```

## Add key: paste a client public key, 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
┌─ 1 Client keys ────────────────────────────────────────────────────────┐╔═ 2 New key · paste pubkey ═════════════════╗
│                                                                        │║                                            ║
│   label (client)      fingerprint                                      │║ Step 2 of 2  ·  paste the client public ke ║
│   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… │║                                            ║
│ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… │║ label (client name)                        ║
│   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… │║  wrangler-tablet▏                          ║
│   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… │║                                            ║
│                                                                        │║ public key                                 ║
│ 4 keys in ~magvaultclient/.ssh/authorized_keys                         │║  ssh-… AAAAC3NzaC1lZDI1NTE5AAAAIOk4Lw9CnT  ║
│                                                                        │║  2eQk7RaZu0VbXdJ5sMfOgP1iNtE8oYqA3c        ║
│                                                                        │║  wrangler-tablet                           ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║ ✓ valid public key                         ║
│                                                                        │║ fingerprint                                ║
│                                                                        │║   SHA256:Hy4Lw9CnT2eQk7RaZu0VbXdJ5sMfOgP1i ║
│                                                                        │║   NtE8oYqA3c                               ║
│                                                                        │║                                            ║
│                                                                        │║ Enter adds it to                           ║
│                                                                        │║ ~magvaultclient/.ssh/authorized_keys. The  ║
│                                                                        │║ client already holds its private key.      ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│                                                                        │║                                            ║
│ Public keys and labels only. Server-generated private                  │║  Enter  Add key    Esc  Cancel             ║
│ keys are never kept after they have been shown.                        │║                                            ║
└────────────────────────────────────────────────────────────────────────┘╚════════════════════════════════════════════╝
 Tab field · Enter add key · Esc cancel (back to list)
```

## Revoke confirm, 120x36

```text
 MAGVAULT  ·  Key module  ·  server (admin)                                                         running as magvault
╔═ 1 Client keys ════════════════════════════════════════════════════════╗┌─ 2 Revoke key ─────────────────────────────┐
║                                                                        ║│                                            │
║   label (client)      fingerprint                                      ║│ label (client)  dit-cart-a                 │
║   lab-laptop-01       SHA256:Zk3pQ8v1mYc0Tn7fLd2uHs9RaXbE4wGjNiK6oPqV… ║│ fingerprint                                │
║ ▶ dit-cart-a          SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAiT3yQmN6… ║│   SHA256:b1Jx7WnC4dRz9sUe0LqMhYt2VfKo8GpAi │
║   edit-suite-02       SHA256:Qe5aHr8UyL1cNvZk3TgW0sOdXjB7mFiP4tYn9ARw… ║│   T3yQmN6xDE                               │
║   wrangler-tablet     SHA256:Tm0Fz6KcVb2oYh9RnJd4LxQe7WsAuG3pNiE1yXqB… ║│                                            │
║                                                                        ║│ ┌────────────────────────────────────────┐ │
║ 4 keys in ~magvaultclient/.ssh/authorized_keys                         ║│ │ Revoke this key?                       │ │
║                                                                        ║│ │  This client will no longer be         │ │
║                                                                        ║│ │  able to connect.                      │ │
║                                                                        ║│ │  It is removed from authorized_keys.   │ │
║                                                                        ║│ │                                        │ │
║                                                                        ║│ │   Enter  Revoke   Esc  Cancel          │ │
║                                                                        ║│ └────────────────────────────────────────┘ │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║                                                                        ║│                                            │
║ Public keys and labels only. Server-generated private                  ║│                                            │
║ keys are never kept after they have been shown.                        ║│                                            │
╚════════════════════════════════════════════════════════════════════════╝└────────────────────────────────────────────┘
 Enter revoke · Esc cancel
```

## Open points for the plan

- **No dashboard entry or letter.** The dashboard module menu and key table in PLAN.md do not list the Key module (i, g, l, e, z, p, v, s are taken). It needs a menu entry and a letter; `k` is free and is assumed here but not drawn on the dashboard.
- **No "added date".** PLAN.md only keeps the public key and a label (client name), so the list shows label and fingerprint only. The fingerprint is derived from the public key.
- **Drive list fields.** PLAN.md says the module "lists removable drives" but not what is shown; the mockup shows a volume name and device only.
- **File location.** PLAN.md does not say where `w` suggests writing; the path in the mockup is a placeholder.
- **Key type.** PLAN.md does not name a key type or size, so none is shown.
- **"Key module restrictions"** is still listed as an open item in PLAN.md.
