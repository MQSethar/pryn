(() => {
  "use strict";

  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
  const root = document.body.dataset.root || ".";
  const repo = "MQSethar/pryn";
  const releaseRoot = `https://github.com/${repo}/releases/download`;
  const fallback = {
    version: "0.1.1",
    tag: "v0.1.1",
    aurPublished: false,
    aurUrl: "https://aur.archlinux.org/packages/pryn-bin",
    pkgbuildUrl: "assets/pryn-bin.PKGBUILD",
    debAsset: "pryn_0.1.1_amd64.deb",
    rpmAsset: "pryn-0.1.1-1.x86_64.rpm",
    archiveAsset: "pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz",
    pkgbuildAsset: "pryn-bin.PKGBUILD",
    checksumsAsset: "SHA256SUMS"
  };
  let release = fallback;
  let toastTimer = 0;

  function setTheme(theme) {
    const selected = theme === "light" ? "light" : "dark";
    document.documentElement.dataset.theme = selected;
    const label = selected === "dark" ? "Light mode" : "Dark mode";
    const button = $("#theme-toggle");
    if (button) {
      button.setAttribute("aria-label", `Switch to ${label.toLowerCase()}`);
      const text = $("#theme-label");
      if (text) text.textContent = label;
    }
    const metaTheme = $("meta[name='theme-color']");
    if (metaTheme) metaTheme.content = selected === "dark" ? "#050608" : "#f4f6fa";
    try { localStorage.setItem("pryn-theme", selected); } catch (_) {}
  }

  function initTheme() {
    let saved = null;
    try { saved = localStorage.getItem("pryn-theme"); } catch (_) {}
    const preferred = window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    setTheme(saved || preferred);
    const button = $("#theme-toggle");
    if (button) {
      button.addEventListener("click", () => {
        setTheme(document.documentElement.dataset.theme === "dark" ? "light" : "dark");
      });
    }
  }

  function notify(message) {
    let toast = $("#toast");
    if (!toast) {
      toast = document.createElement("div");
      toast.id = "toast";
      toast.setAttribute("role", "status");
      toast.setAttribute("aria-live", "polite");
      document.body.append(toast);
    }
    toast.textContent = message;
    toast.classList.add("visible");
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => toast.classList.remove("visible"), 1800);
  }

  async function copyText(value) {
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(value);
      } else {
        const input = document.createElement("textarea");
        input.value = value;
        input.setAttribute("readonly", "");
        input.style.position = "fixed";
        input.style.opacity = "0";
        document.body.append(input);
        input.select();
        const copied = document.execCommand("copy");
        input.remove();
        if (!copied) throw new Error("Copy unavailable");
      }
      notify("Copied to clipboard");
    } catch (_) {
      notify("Copy failed. Select the command and copy it manually.");
    }
  }

  function initCopyButtons() {
    $$("[data-copy]").forEach(button => {
      button.addEventListener("click", () => {
        const target = document.getElementById(button.dataset.copy);
        if (target) copyText(target.textContent.trim());
      });
    });
  }

  function initMobileMenu() {
    const button = $("#mobile-menu");
    const nav = $("#main-nav");
    if (!button || !nav) return;
    button.addEventListener("click", () => {
      const open = nav.classList.toggle("is-open");
      button.setAttribute("aria-expanded", String(open));
      button.setAttribute("aria-label", open ? "Close menu" : "Open menu");
    });
    $$("#main-nav a").forEach(link => link.addEventListener("click", () => {
      nav.classList.remove("is-open");
      button.setAttribute("aria-expanded", "false");
    }));
  }

  function setMode(panel, mode) {
    $$("[data-mode-button]", panel).forEach(button => {
      const active = button.dataset.modeButton === mode;
      button.classList.toggle("active", active);
      button.setAttribute("aria-selected", String(active));
    });
    $$("[data-mode-panel]", panel).forEach(content => {
      const active = content.dataset.modePanel === mode;
      content.hidden = !active;
      content.classList.toggle("active", active);
    });
  }

  function initModeTabs() {
    $$(".platform-panel").forEach(panel => {
      $$("[data-mode-button]", panel).forEach(button => {
        button.addEventListener("click", () => setMode(panel, button.dataset.modeButton));
      });
    });
    const cargo = $(".cargo-section");
    if (cargo) {
      // Cargo actions are shown together; no extra tabs are needed.
    }
  }

  function initPlatformTabs() {
    const tabs = $$(".platform-tab");
    if (!tabs.length) return;
    function activate(tab) {
      tabs.forEach(item => {
        const active = item === tab;
        item.classList.toggle("active", active);
        item.setAttribute("aria-selected", String(active));
        item.tabIndex = active ? 0 : -1;
      });
      $$(".platform-panel").forEach(panel => {
        const active = panel.dataset.platformPanel === tab.dataset.platform;
        panel.hidden = !active;
        panel.classList.toggle("active", active);
      });
    }
    tabs.forEach((tab, index) => {
      tab.addEventListener("click", () => activate(tab));
      tab.addEventListener("keydown", event => {
        let next = index;
        if (event.key === "ArrowRight" || event.key === "ArrowDown") next = (index + 1) % tabs.length;
        else if (event.key === "ArrowLeft" || event.key === "ArrowUp") next = (index - 1 + tabs.length) % tabs.length;
        else if (event.key === "Home") next = 0;
        else if (event.key === "End") next = tabs.length - 1;
        else return;
        event.preventDefault();
        activate(tabs[next]);
        tabs[next].focus();
      });
    });
    tabs.forEach((tab, index) => { tab.tabIndex = index === 0 ? 0 : -1; });
  }

  function buildCommands() {
    const base = `${releaseRoot}/${release.tag}`;
    const deb = release.debAsset;
    const rpm = release.rpmAsset;
    const archive = release.archiveAsset;
    const pkgbuild = release.pkgbuildAsset;

    const commands = {
      "debian-install": `cd ~/Downloads && sudo apt install ./${deb}`,
      "debian-update": `cd ~/Downloads && sudo apt install ./${deb}`,
      "debian-remove": "sudo apt remove pryn",
      "fedora-install": `cd ~/Downloads && sudo dnf install --nogpgcheck ./${rpm}`,
      "fedora-update": `cd ~/Downloads && sudo dnf install --nogpgcheck ./${rpm}`,
      "fedora-remove": "sudo dnf remove pryn",
      "opensuse-install": `cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./${rpm}`,
      "opensuse-update": `cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./${rpm}`,
      "opensuse-remove": "sudo zypper remove pryn",
      "arch-install": `sudo pacman -S --needed base-devel\nmkdir -p ~/build/pryn-bin\ncp ~/Downloads/${pkgbuild} ~/build/pryn-bin/PKGBUILD\ncd ~/build/pryn-bin\nmakepkg -si`,
      "arch-update": `cp -f ~/Downloads/${pkgbuild} ~/build/pryn-bin/PKGBUILD\ncd ~/build/pryn-bin\nmakepkg -si`,
      "arch-remove": "sudo pacman -Rns pryn-bin",
      "aur-install": "yay -S pryn-bin",
      "aur-update": "yay -Syu pryn-bin",
      "aur-remove": "yay -Rns pryn-bin",
      "other-install": `mkdir -p ~/.local/bin\ntar -xzf ~/Downloads/${archive} -C /tmp\ninstall -Dm755 /tmp/pryn-v${release.version}/pryn ~/.local/bin/pryn`,
      "other-update": `tar -xzf ~/Downloads/${archive} -C /tmp\ninstall -Dm755 /tmp/pryn-v${release.version}/pryn ~/.local/bin/pryn`,
      "other-remove": "rm -f ~/.local/bin/pryn",
      "cargo-install": "cargo install pryn --locked",
      "cargo-update": "cargo install pryn --locked --force",
      "cargo-remove": "cargo uninstall pryn",
      "first-run": "pryn --version && pryn check"
    };

    Object.entries(commands).forEach(([id, command]) => {
      const node = document.getElementById(id);
      if (node) node.textContent = command;
    });

    const files = {
      deb: deb,
      rpm: rpm,
      archive: archive,
      pkgbuild: pkgbuild
    };
    $$("[data-file]").forEach(node => {
      const value = files[node.dataset.file];
      if (value) node.textContent = value;
    });
    $$("[data-asset]").forEach(link => {
      const value = files[link.dataset.asset];
      if (!value) return;
      if (link.dataset.asset === "pkgbuild" && release.pkgbuildUrl) {
        link.href = `${root}/${release.pkgbuildUrl}`;
      } else {
        link.href = `${base}/${value}`;
      }
    });

    const versionNode = $("#installer-version");
    if (versionNode) versionNode.textContent = release.version;
    const chip = $("#release-chip");
    if (chip) chip.textContent = `v${release.version}`;

    const manual = $("#arch-manual");
    const aur = $("#arch-aur");
    if (manual && aur) {
      if (release.aurPublished === true) {
        manual.hidden = true;
        aur.hidden = false;
        const status = $("#arch-status");
        if (status) {
          status.textContent = "Available in AUR";
          status.classList.remove("status-label");
          status.classList.add("tested-label");
        }
        const link = $("#aur-link");
        if (link) link.href = release.aurUrl || fallback.aurUrl;
      } else {
        manual.hidden = false;
        aur.hidden = true;
      }
    }
  }

  async function loadRelease() {
    try {
      const response = await fetch(`${root}/release.json`, { cache: "no-store" });
      if (!response.ok) throw new Error("Release metadata unavailable");
      const data = await response.json();
      if (!/^\d+\.\d+\.\d+$/.test(data.version) || !/^v\d+\.\d+\.\d+$/.test(data.tag)) {
        throw new Error("Invalid release metadata");
      }
      release = { ...fallback, ...data };
    } catch (_) {
      release = fallback;
    }
    buildCommands();
  }

  initTheme();
  initMobileMenu();
  initCopyButtons();
  initPlatformTabs();
  initModeTabs();
  loadRelease();
})();
