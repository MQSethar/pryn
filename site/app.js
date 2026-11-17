(() => {
  "use strict";

  const fallback = {
    version: "0.1.1",
    tag: "v0.1.1",
    architecture: "x86_64",
    aurPublished: false,
    aurUrl: "https://aur.archlinux.org/packages/pryn-bin",
    debAsset: "pryn_0.1.1_amd64.deb",
    rpmAsset: "pryn-0.1.1-1.x86_64.rpm",
    archiveAsset: "pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz",
    pkgbuildAsset: "pryn-bin.PKGBUILD",
    checksumsAsset: "SHA256SUMS"
  };
  const repo = "MQSethar/pryn";
  const releaseRoot = `https://github.com/${repo}/releases/download`;
  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
  let release = fallback;
  let toastTimer = 0;

  function setTheme(theme) {
    const selected = theme === "light" ? "light" : "dark";
    document.documentElement.dataset.theme = selected;
    const button = $("#theme-toggle");
    const label = selected === "dark" ? "Light mode" : "Dark mode";
    button.setAttribute("aria-label", `Switch to ${label.toLowerCase()}`);
    $("#theme-label").textContent = label;
    const metaTheme = $("meta[name='theme-color']");
    if (metaTheme) metaTheme.content = selected === "dark" ? "#050608" : "#f5f6f8";
    try { localStorage.setItem("pryn-theme", selected); } catch (_) { /* Storage can be disabled. */ }
  }

  function initTheme() {
    let saved = null;
    try { saved = localStorage.getItem("pryn-theme"); } catch (_) { /* Storage can be disabled. */ }
    const preferred = window.matchMedia && window.matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
    setTheme(saved || preferred);
    $("#theme-toggle").addEventListener("click", () => {
      setTheme(document.documentElement.dataset.theme === "dark" ? "light" : "dark");
    });
  }

  function notify(message) {
    const toast = $("#toast");
    toast.textContent = message;
    toast.classList.add("is-visible");
    window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => toast.classList.remove("is-visible"), 1800);
  }

  async function copy(value) {
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
        if (!copied) throw new Error("copy unavailable");
      }
      notify("Command copied");
    } catch (_) {
      notify("Copy failed. Select the command and copy it manually.");
    }
  }

  function initCopyButtons() {
    $$('[data-copy]').forEach(button => {
      button.addEventListener("click", () => {
        const target = $(`[data-command="${button.dataset.copy}"]`);
        if (target) copy(target.textContent.trim());
      });
    });
  }

  function initTabs() {
    const tabs = $$(".distro-tab");
    function activate(tab) {
      tabs.forEach(item => {
        const selected = item === tab;
        item.classList.toggle("is-active", selected);
        item.setAttribute("aria-selected", String(selected));
        item.tabIndex = selected ? 0 : -1;
      });
      $$(".distro-panel").forEach(panel => {
        const selected = panel.dataset.panel === tab.dataset.tab;
        panel.classList.toggle("is-active", selected);
        panel.hidden = !selected;
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

  function initMobileMenu() {
    const button = $("#mobile-menu");
    const nav = $("#main-nav");
    button.addEventListener("click", () => {
      const open = nav.classList.toggle("is-open");
      button.setAttribute("aria-expanded", String(open));
      button.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
    });
    $$("#main-nav a").forEach(link => link.addEventListener("click", () => {
      nav.classList.remove("is-open");
      button.setAttribute("aria-expanded", "false");
    }));
  }

  function configureRelease() {
    const base = `${releaseRoot}/${release.tag}`;
    const assets = {
      deb: release.debAsset,
      rpm: release.rpmAsset,
      archive: release.archiveAsset,
      pkgbuild: release.pkgbuildAsset
    };
    const commands = {
      debian: `cd ~/Downloads && sudo apt install ./${release.debAsset}`,
      fedora: `cd ~/Downloads && sudo dnf install --nogpgcheck ./${release.rpmAsset}`,
      opensuse: `cd ~/Downloads && sudo zypper install --allow-unsigned-rpm ./${release.rpmAsset}`,
      arch: `sudo pacman -S --needed base-devel\nmkdir -p ~/build/pryn-bin\ncp ~/Downloads/${release.pkgbuildAsset} ~/build/pryn-bin/PKGBUILD\ncd ~/build/pryn-bin\nmakepkg --printsrcinfo > .SRCINFO\nmakepkg --verifysource\nmakepkg -si`,
      other: `cd ~/Downloads && tar -xzf ${release.archiveAsset} && install -Dm755 pryn-v${release.version}/pryn ~/.local/bin/pryn`,
      aur: "yay -S pryn-bin",
      checksum: `cd ~/Downloads && curl -fLO ${base}/${release.checksumsAsset} && sha256sum --ignore-missing --check ${release.checksumsAsset}`
    };

    $("#release-chip").textContent = `v${release.version}`;
    $("#installer-version").textContent = release.version;
    $$('[data-file="deb"]').forEach(node => { node.textContent = release.debAsset; });
    $$('[data-file="rpm"]').forEach(node => { node.textContent = release.rpmAsset; });
    $$('[data-file="archive"]').forEach(node => { node.textContent = release.archiveAsset; });
    $$('[data-file="pkgbuild"]').forEach(node => { node.textContent = release.pkgbuildAsset; });
    $$('[data-asset]').forEach(link => { link.href = `${base}/${assets[link.dataset.asset]}`; });
    Object.entries(commands).forEach(([key, value]) => {
      const code = $(`[data-command="${key}"]`);
      if (code) code.textContent = value;
    });

    if (release.aurPublished === true) {
      $("#arch-status").textContent = "Available in AUR";
      $("#arch-status").classList.remove("pending-label");
      $("#arch-status").classList.add("test-label");
      $("#arch-manual").hidden = true;
      $("#arch-aur").hidden = false;
      $("#aur-link").href = release.aurUrl || fallback.aurUrl;
    } else {
      $("#arch-manual").hidden = false;
      $("#arch-aur").hidden = true;
    }
  }

  async function loadRelease() {
    try {
      const response = await fetch("./release.json", { cache: "no-store" });
      if (!response.ok) throw new Error("Release metadata unavailable");
      const current = await response.json();
      if (!/^\d+\.\d+\.\d+$/.test(current.version) || !/^v\d+\.\d+\.\d+$/.test(current.tag)) {
        throw new Error("Invalid release metadata");
      }
      release = { ...fallback, ...current };
    } catch (_) {
      release = fallback;
    }
    configureRelease();
  }

  initTheme();
  initTabs();
  initCopyButtons();
  initMobileMenu();
  loadRelease();
})();
