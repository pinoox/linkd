document.addEventListener('DOMContentLoaded', () => {
  initInstallTabs();
  initCopyButtons();
  initEcosystemTabs();
  initTuiSimulator();
  initCliSearch();
});

// Install Command Tabs
function initInstallTabs() {
  const tabs = document.querySelectorAll('.install-tab');
  const codeElem = document.querySelector('.install-code');

  const commands = {
    bash: 'curl -fsSL https://raw.githubusercontent.com/pinoox/linkd/main/install.sh | bash',
    powershell: 'irm https://raw.githubusercontent.com/pinoox/linkd/main/install.ps1 | iex',
    cargo: 'cargo install --path crates/linkd-cli --locked'
  };

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      const target = tab.getAttribute('data-target');
      if (commands[target] && codeElem) {
        codeElem.textContent = commands[target];
      }
    });
  });
}

// Copy Buttons
function initCopyButtons() {
  document.querySelectorAll('.copy-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      const targetSelector = btn.getAttribute('data-copy-target');
      let textToCopy = '';
      if (targetSelector) {
        const targetEl = document.querySelector(targetSelector);
        textToCopy = targetEl ? targetEl.textContent.trim() : '';
      } else {
        textToCopy = btn.getAttribute('data-text') || '';
      }

      if (textToCopy) {
        navigator.clipboard.writeText(textToCopy).then(() => {
          const originalHTML = btn.innerHTML;
          btn.innerHTML = `✓ Copied!`;
          btn.classList.add('copied');
          setTimeout(() => {
            btn.innerHTML = originalHTML;
            btn.classList.remove('copied');
          }, 2000);
        });
      }
    });
  });
}

// Ecosystem Explorer Data & Switching
const ECOSYSTEM_DATA = {
  npm: {
    title: '🟢 JavaScript & TypeScript',
    desc: 'Deep integration with npm, pnpm, yarn, and bun. Automatically detects monorepo package roots, links into node_modules, and creates isolated project-local shadow directories for pnpm to prevent global store corruption.',
    features: [
      'Automatic package.json name & main entry resolution',
      'pnpm shadow isolation: never writes to ~/.pnpm-store',
      'Watches package-lock.json, pnpm-lock.yaml, yarn.lock, and bun.lockb',
      'Zero manifest edits: git status remains completely clean'
    ],
    code: `# 1. Link your UI kit into your web app
linkd link ./packages/ui-kit ./apps/web-app

# Reconciler automatically links:
# ./packages/ui-kit → ./apps/web-app/node_modules/@acme/ui-kit

# Run installs without breaking links!
cd ./apps/web-app && pnpm install`
  },
  composer: {
    title: '🐘 PHP (Composer)',
    desc: 'Seamless local PHP package development. Automatically parses vendor/package namespaces from composer.json, routes to consumer/vendor, and notifies you when composer dump-autoload is needed for new classmaps.',
    features: [
      'Namespaced vendor target path resolution',
      'Watches composer.lock and vendor/composer/installed.json',
      'Survives composer update and composer install',
      'Smart hints for PHP classmap and autoload dumping'
    ],
    code: `# Link a local Composer library into a Laravel / Symfony app
linkd link ./packages/acme-logger ./apps/laravel-api

# Target resolved to:
# ./apps/laravel-api/vendor/acme/logger

# When you add a new class:
# linkd daemon detects it and provides autoload hints`
  },
  python: {
    title: '🐍 Python (uv / pip / poetry)',
    desc: 'Works seamlessly with uv, pip, Poetry, and Flit. Automatically discovers virtual environments (.venv/venv), resolves site-packages, and filters bytecode/test caches.',
    features: [
      'Parses PEP 621 pyproject.toml, setup.py, and setup.cfg',
      'Auto-detects .venv site-packages on Windows & Unix',
      'Watches uv.lock, poetry.lock, Pipfile.lock, requirements.txt',
      'Automatically filters __pycache__, *.pyc, and .pytest_cache'
    ],
    code: `# Link a Python ML/data library into an API service
linkd link ./packages/ml-core ./apps/fastapi-app

# Resolved target:
# ./apps/fastapi-app/.venv/Lib/site-packages/ml_core

# Survives 'uv sync' or 'pip install -r requirements.txt'`
  },
  go: {
    title: '🐹 Go (Go Modules & Vendor)',
    desc: 'Enables smooth multi-module local development by syncing Go packages directly into consumer vendor directories according to the go.mod module path declaration.',
    features: [
      'Extracts module path directly from source go.mod',
      'Syncs to vendor/<module_path> structure',
      'Watches go.sum, go.work, and vendor/modules.txt',
      'Compatible with standard "go build -mod=vendor"'
    ],
    code: `# Link a Go shared module into a microservice
linkd link ./packages/auth-lib ./apps/auth-service

# Destination:
# ./apps/auth-service/vendor/example.com/acme/auth-lib

# Build using vendor cache:
go build -mod=vendor ./...`
  },
  cargo: {
    title: '🦀 Rust (Cargo)',
    desc: 'Link local Rust crates into application vendor directories without modifying source Cargo.toml manifests or committing local path dependencies to Git.',
    features: [
      'Parses [package.name] directly from Cargo.toml',
      'Syncs to consumer/vendor/<crate_name>',
      'Watches Cargo.lock and .cargo/config.toml',
      'Filters target/ build artifacts and debug caches'
    ],
    code: `# Link a local crate into a Rust workspace or binary app
linkd link ./crates/protocol-core ./apps/rust-server

# Destination:
# ./apps/rust-server/vendor/protocol-core

# Compiles immediately without editing Cargo.toml`
  },
  jvm: {
    title: '☕ Java & Kotlin (JVM)',
    desc: 'Supports Maven and Gradle multi-project setups. Parses group and artifact IDs from pom.xml or build.gradle, syncing directly into libs/ or Maven Local repository.',
    features: [
      'Parses groupId and artifactId from pom.xml / build.gradle',
      'Syncs to project-local libs/ or ~/.m2/repository',
      'Watches pom.xml, build.gradle, and gradle.lockfile',
      'Filters build/, target/, .gradle/, and *.class files'
    ],
    code: `# Link a Java/Kotlin library into an application
linkd link ./packages/java-sdk ./apps/spring-service

# Destination:
# ./apps/spring-service/libs/java-sdk

# gradle build --refresh-dependencies picks up all changes`
  },
  custom: {
    title: '📁 Custom Paths (Any Framework)',
    desc: 'Total flexibility. Sync any arbitrary directory into any target destination with automated path loop-guarding to prevent circular watching.',
    features: [
      'Works with any folder, framework, or asset directory',
      'Loop guard rejects nested source/target hierarchies',
      'Provenance tracking via .linkd-marker.json',
      'Atomic directory swapping and reflink acceleration'
    ],
    code: `# Custom folder-to-folder link
linkd link ./shared-assets ./apps/electron-app \\
  --target ./apps/electron-app/src/assets/shared

# Full real-time background sync with zero loop hazards`
  }
};

function initEcosystemTabs() {
  const tabs = document.querySelectorAll('.eco-btn');
  const titleEl = document.getElementById('eco-title');
  const descEl = document.getElementById('eco-desc');
  const featuresEl = document.getElementById('eco-features');
  const codeEl = document.getElementById('eco-code');

  tabs.forEach(tab => {
    tab.addEventListener('click', () => {
      tabs.forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      const key = tab.getAttribute('data-eco');
      const data = ECOSYSTEM_DATA[key];

      if (data) {
        titleEl.textContent = data.title;
        descEl.textContent = data.desc;
        featuresEl.innerHTML = data.features.map(f => `<li><span class="t-green">✓</span> ${f}</li>`).join('');
        codeEl.textContent = data.code;
      }
    });
  });
}

// Interactive TUI Simulator
const TUI_LINKS = [
  {
    name: '@acme/ui-kit',
    eco: '[npm]',
    ecoColor: 'var(--accent-red)',
    status: '[SYNCED]',
    statusClass: 't-green',
    source: 'C:/projects/packages/ui-kit',
    target: 'C:/projects/apps/web/node_modules/@acme/ui-kit',
    pm: 'pnpm (shadow isolation)',
    files: 42,
    syncs: 18
  },
  {
    name: 'acme/logger',
    eco: '[composer]',
    ecoColor: 'var(--accent-purple)',
    status: '[SYNCED]',
    statusClass: 't-green',
    source: 'C:/projects/packages/acme-logger',
    target: 'C:/projects/apps/api/vendor/acme/logger',
    pm: 'composer',
    files: 14,
    syncs: 6
  },
  {
    name: 'ml-core',
    eco: '[python]',
    ecoColor: 'var(--accent-yellow)',
    status: '[SYNCED]',
    statusClass: 't-green',
    source: 'C:/projects/packages/ml-core',
    target: 'C:/projects/apps/api/.venv/Lib/site-packages/ml_core',
    pm: 'uv / pip',
    files: 28,
    syncs: 9
  },
  {
    name: 'example.com/auth',
    eco: '[go]',
    ecoColor: 'var(--accent-cyan)',
    status: '[PAUSED]',
    statusClass: 't-yellow',
    source: 'C:/projects/packages/auth',
    target: 'C:/projects/apps/gateway/vendor/example.com/auth',
    pm: 'go modules',
    files: 19,
    syncs: 4
  }
];

function initTuiSimulator() {
  const listEl = document.getElementById('tui-links-list');
  const inspEl = document.getElementById('tui-inspector-content');
  const logsEl = document.getElementById('tui-logs-content');

  let selectedIdx = 0;

  function renderList() {
    listEl.innerHTML = '';
    TUI_LINKS.forEach((link, i) => {
      const item = document.createElement('div');
      item.className = `tui-link-item ${i === selectedIdx ? 'selected' : ''}`;
      item.innerHTML = `
        <div>
          <span style="color: ${link.ecoColor}">${link.eco}</span>
          <span style="font-weight: 600; color: #fff; margin-left: 6px">${link.name}</span>
        </div>
        <span class="${link.statusClass}">${link.status}</span>
      `;
      item.addEventListener('click', () => {
        selectedIdx = i;
        renderList();
        renderInspector();
      });
      listEl.appendChild(item);
    });
  }

  function renderInspector() {
    const l = TUI_LINKS[selectedIdx];
    inspEl.innerHTML = `
      <div style="color: var(--accent-cyan); font-weight: bold; margin-bottom: 4px">Package: ${l.name} ${l.eco}</div>
      <div>Source: <span style="color: #fff">${l.source}</span></div>
      <div>Target: <span style="color: #fff">${l.target}</span></div>
      <div>Package Manager: <span class="t-magenta">${l.pm}</span> | Files: <span class="t-cyan">${l.files}</span> | Reconciles: <span class="t-green">${l.syncs}</span></div>
    `;
  }

  function appendLog(msg, colorClass = 't-cyan') {
    const time = new Date().toLocaleTimeString();
    const logItem = document.createElement('div');
    logItem.innerHTML = `<span class="t-dim">[${time}]</span> <span class="${colorClass}">${msg}</span>`;
    logsEl.appendChild(logItem);
    logsEl.scrollTop = logsEl.scrollHeight;
  }

  // Hotkey simulation
  document.addEventListener('keydown', (e) => {
    if (document.activeElement.tagName === 'INPUT') return;

    if (e.key === 'r' || e.key === 'R') {
      const l = TUI_LINKS[selectedIdx];
      l.syncs++;
      appendLog(`Sync triggered for ${l.name} (${l.files} files reconciled)`, 't-green');
      renderInspector();
    } else if (e.key === ' ') {
      e.preventDefault();
      const l = TUI_LINKS[selectedIdx];
      if (l.status === '[SYNCED]') {
        l.status = '[PAUSED]';
        l.statusClass = 't-yellow';
        appendLog(`Paused sync on ${l.name}`, 't-yellow');
      } else {
        l.status = '[SYNCED]';
        l.statusClass = 't-green';
        appendLog(`Resumed sync on ${l.name}`, 't-green');
      }
      renderList();
    } else if (e.key === 'c' || e.key === 'C') {
      logsEl.innerHTML = '<div class="t-dim">Logs cleared.</div>';
    }
  });

  renderList();
  renderInspector();
}

// CLI Table Search
function initCliSearch() {
  const searchInput = document.getElementById('cli-filter');
  const tableRows = document.querySelectorAll('.cli-table tbody tr');

  if (!searchInput) return;

  searchInput.addEventListener('input', () => {
    const query = searchInput.value.toLowerCase().trim();
    tableRows.forEach(row => {
      const text = row.textContent.toLowerCase();
      row.style.display = text.includes(query) ? '' : 'none';
    });
  });
}
