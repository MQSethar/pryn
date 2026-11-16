(() => {
  "use strict";

  const fallback = {
    version: "0.1.1",
    tag: "v0.1.1",
    architecture: "x86_64",
    aurPublished: false,
    debAsset: "pryn_0.1.1_amd64.deb",
    rpmAsset: "pryn-0.1.1-1.x86_64.rpm",
    archiveAsset: "pryn-v0.1.1-x86_64-unknown-linux-musl.tar.gz",
    pkgbuildAsset: "pryn-bin.PKGBUILD",
    checksumsAsset: "SHA256SUMS"
  };
  const ownerRepo = "MQSethar/pryn";
  const releaseBase = `https://github.com/${ownerRepo}/releases/download`;
  const $ = (selector, root = document) => root.querySelector(selector);
  const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];

  let config = fallback;
  const commands = {};

  function buildCommands() {
    const v = config.version;
    const base = `${releaseBase}/${config.tag}`;
    const deb = `${base}/${config.debAsset}`;
    const rpm = `${base}/${config.rpmAsset}`;
    const archive = `${base}/${config.archiveAsset}`;
    const recipe = `${base}/${config.pkgbuildAsset}`;
    const sums = `${base}/${config.checksumsAsset}`;

    commands.debian = `curl -fLO ${deb}\nsudo apt install ./${config.debAsset}`;
    commands.fedora = `curl -fLO ${rpm}\nsudo dnf install --nogpgcheck ./${config.rpmAsset}`;
    commands.opensuse = `sudo zypper install --allow-unsigned-rpm ./${config.rpmAsset}`;
    commands.arch = config.aurPublished
      ? "yay -S pryn-bin"
      : `sudo pacman -S --needed base-devel curl\nmkdir -p ~/build/pryn-bin && cd ~/build/pryn-bin\ncurl -fL ${recipe} -o PKGBUILD\nmakepkg --printsrcinfo > .SRCINFO\nmakepkg --verifysource\nmakepkg -si`;
    commands.generic = `curl -fLO ${archive}\ntar -xzf ${config.archiveAsset}\nmkdir -p "$HOME/.local/bin"\ninstall -m 755 pryn-v${v}/pryn "$HOME/.local/bin/pryn"`;
    commands.checksum = `curl -fLO ${sums}\nsha256sum --ignore-missing --check ${config.checksumsAsset}`;

    $("#version-label").textContent = `V${v}`;
    $("#console-version").textContent = v;
    $$("[data-version]").forEach(node => node.textContent = v);

    const links = {
      deb: config.debAsset,
      rpm: config.rpmAsset,
      archive: config.archiveAsset,
      pkgbuild: config.pkgbuildAsset
    };
    $$("[data-asset]").forEach(anchor => {
      anchor.href = `${base}/${links[anchor.dataset.asset]}`;
    });

    Object.entries(commands).forEach(([key, value]) => {
      const target = $(`#command-${key}`);
      if (target) target.textContent = value;
    });

    const aurTab = $("#tab-arch");
    const archNote = $(".callout");
    if (config.aurPublished) {
      aurTab.querySelector("small").textContent = "pacman · makepkg · AUR";
      $(".pending-tag").textContent = "AVAILABLE IN AUR";
      $(".pending-tag").classList.remove("pending-tag");
      $(".pending-tag").classList.add("tested-tag");
      if (archNote) archNote.innerHTML = "<b>AUR package published.</b><span>Install the reviewed community package with <code>yay -S pryn-bin</code>.</span>";
    } else {
      aurTab.querySelector("small").textContent = "local makepkg · AUR pending";
    }
  }

  async function loadConfig() {
    try {
      const response = await fetch("./release.json", { cache: "no-store" });
      if (!response.ok) throw new Error("release configuration unavailable");
      const live = await response.json();
      if (typeof live.version === "string" && /^\d+\.\d+\.\d+$/.test(live.version)) {
        config = { ...fallback, ...live };
      }
    } catch (_) {
      config = fallback;
    }
    buildCommands();
  }

  function showToast(message) {
    const toast = $("#toast");
    toast.textContent = message;
    toast.classList.add("show");
    window.clearTimeout(showToast.timer);
    showToast.timer = window.setTimeout(() => toast.classList.remove("show"), 1800);
  }

  async function copyText(value) {
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(value);
      } else {
        const area = document.createElement("textarea");
        area.value = value;
        area.setAttribute("readonly", "");
        area.style.position = "fixed";
        area.style.opacity = "0";
        document.body.append(area);
        area.select();
        const copied = document.execCommand("copy");
        area.remove();
        if (!copied) throw new Error("copy failed");
      }
      showToast("Command copied to clipboard");
    } catch (_) {
      showToast("Copy unavailable. Select the command manually.");
    }
  }

  function initTabs() {
    const tabs = $$(".distro-tab");
    function activate(tab) {
      tabs.forEach(item => {
        const selected = item === tab;
        item.classList.toggle("active", selected);
        item.setAttribute("aria-selected", String(selected));
        item.tabIndex = selected ? 0 : -1;
      });
      $$(".install-panel").forEach(panel => {
        const active = panel.dataset.panel === tab.dataset.tab;
        panel.classList.toggle("active", active);
        panel.hidden = !active;
      });
    }
    tabs.forEach((tab, index) => {
      tab.addEventListener("click", () => activate(tab));
      tab.addEventListener("keydown", event => {
        let next = index;
        if (event.key === "ArrowDown" || event.key === "ArrowRight") next = (index + 1) % tabs.length;
        else if (event.key === "ArrowUp" || event.key === "ArrowLeft") next = (index - 1 + tabs.length) % tabs.length;
        else if (event.key === "Home") next = 0;
        else if (event.key === "End") next = tabs.length - 1;
        else return;
        event.preventDefault();
        activate(tabs[next]);
        tabs[next].focus();
      });
    });
    tabs.forEach((tab, i) => { tab.tabIndex = i === 0 ? 0 : -1; });
  }

  function initCopyButtons() {
    $$("[data-copy]").forEach(button => {
      button.addEventListener("click", () => {
        const key = button.dataset.copy;
        if (commands[key]) copyText(commands[key]);
      });
    });
  }

  function initMobileMenu() {
    const toggle = $(".menu-toggle");
    const nav = $(".topnav");
    toggle.addEventListener("click", () => {
      const open = nav.classList.toggle("open");
      toggle.setAttribute("aria-expanded", String(open));
      toggle.setAttribute("aria-label", open ? "Close navigation" : "Open navigation");
    });
    $$(".topnav a").forEach(link => link.addEventListener("click", () => {
      nav.classList.remove("open");
      toggle.setAttribute("aria-expanded", "false");
    }));
  }

  initTabs();
  initCopyButtons();
  initMobileMenu();
  loadConfig();
})();
