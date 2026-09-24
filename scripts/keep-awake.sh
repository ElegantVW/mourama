#!/usr/bin/env bash
# Keep VANGUARDA awake so citânias keep breathing.
# Needs root for systemd drop-ins and masking sleep targets.
set -euo pipefail

if [[ "$(id -u)" -ne 0 ]]; then
  echo "mourama: keep-awake needs root to tell the box it does not sleep." >&2
  echo "  next:  sudo $0" >&2
  exit 1
fi

install -d /etc/systemd/sleep.conf.d /etc/systemd/logind.conf.d

cat >/etc/systemd/sleep.conf.d/mourama.conf <<'EOF'
[Sleep]
AllowSuspend=no
AllowHibernation=no
AllowHybridSleep=no
AllowSuspendThenHibernate=no
EOF

cat >/etc/systemd/logind.conf.d/mourama.conf <<'EOF'
[Login]
IdleAction=ignore
HandleLidSwitch=ignore
HandleLidSwitchExternalPower=ignore
HandleLidSwitchDocked=ignore
HandleSuspendKey=ignore
HandleHibernateKey=ignore
HandleSuspendKeyLongPress=ignore
EOF

systemctl mask sleep.target suspend.target hibernate.target hybrid-sleep.target
systemctl restart systemd-logind.service

echo "mourama: sleep is masked. Seal lock still works."
echo "  next:  lock with Mod+Ctrl+l — skip Mod+Ctrl+Shift+l (suspend)."
