import { createHash } from 'node:crypto';
import { mkdtemp, readFile, readdir, rm, stat, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import ts from 'typescript';
import { LEGACY_SKILLS, PAGE_ROUTES, SITE_URL, SKILLS_SOURCE, PLUGIN_NAME, MARKETPLACE_NAME, MARKETPLACE_COMMAND, PLUGIN_COMMAND, PLUGIN_UPDATE_COMMAND, routeFile } from './site-contract.mjs';

const workspace = fileURLToPath(new URL('../', import.meta.url));
const fixturePath = new URL('../tests/fixtures/published-files.json', import.meta.url);
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

const decodeEntities = value => value.replace(/&(?:amp|quot|apos|lt|gt|#39);/g, entity => ({
  '&amp;': '&', '&quot;': '"', '&apos;': "'", '&#39;': "'", '&lt;': '<', '&gt;': '>',
})[entity]);

export function htmlAttributes(source) {
  const attributes = {};
  const pattern = /([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g;
  for (const match of source.matchAll(pattern)) {
    attributes[match[1].toLowerCase()] = decodeEntities(match[2] ?? match[3] ?? match[4] ?? '');
  }
  return attributes;
}

export function htmlElements(html) {
  // Do not treat example code, JavaScript strings or comments as live HTML elements.
  const markup = html.replace(/<!--[\s\S]*?-->|<(script|style)\b[^>]*>[\s\S]*?<\/\1\s*>/gi, match =>
    match.startsWith('<!--') ? '' : match.slice(0, match.indexOf('>') + 1));
  return [...markup.matchAll(/<([a-z][\w:-]*)\b([^>]*?)>/gi)]
    .map(match => ({ tag: match[1].toLowerCase(), attributes: htmlAttributes(match[2]) }));
}

export async function assertPreservedFiles(directory, baseline) {
  for (const [relative, expected] of Object.entries(baseline)) {
    const actual = digest(await readFile(path.join(directory, relative)));
    if (actual !== expected) throw new Error(`Published bytes changed: ${relative}`);
  }
}

export async function assertZipChecksums(directory) {
  const entries = await readdir(path.join(directory, 'downloads'));
  const archives = entries.filter(name => name.endsWith('.zip'));
  if (archives.length < 2) throw new Error('Both published ZIP packages are required');
  for (const archive of archives) {
    const checksum = await readFile(path.join(directory, 'downloads', `${archive}.sha256`), 'utf8');
    const match = checksum.trim().match(/^([a-f0-9]{64})\s+\*?([^\r\n]+)$/i);
    if (!match || match[2] !== archive) throw new Error(`Invalid checksum file: ${archive}.sha256`);
    if (digest(await readFile(path.join(directory, 'downloads', archive))) !== match[1].toLowerCase()) {
      throw new Error(`ZIP checksum mismatch: ${archive}`);
    }
  }
  return archives.length;
}

async function htmlFiles(directory, prefix = '') {
  const results = [];
  for (const item of await readdir(directory, { withFileTypes: true })) {
    const relative = `${prefix}${item.name}`;
    if (item.isDirectory()) results.push(...await htmlFiles(path.join(directory, item.name), `${relative}/`));
    else if (item.name.endsWith('.html') && !relative.startsWith('skills/unattended/templates/')) results.push(relative);
  }
  return results;
}

async function existsFile(file) {
  try { return (await stat(file)).isFile(); } catch { return false; }
}

export async function assertInternalLinks(directory) {
  const documents = new Map();
  for (const relative of await htmlFiles(directory)) {
    const elements = htmlElements(await readFile(path.join(directory, relative), 'utf8'));
    const ids = elements.flatMap(element => element.attributes.id ? [element.attributes.id] : []);
    if (new Set(ids).size !== ids.length) throw new Error(`Duplicate HTML id in ${relative}`);
    documents.set(relative, { elements, ids: new Set(ids) });
  }
  for (const [relative, document] of documents) {
    const route = relative.endsWith('index.html') ? `/${relative.slice(0, -10)}` : `/${relative}`;
    for (const { attributes } of document.elements) {
      for (const attribute of ['href', 'src']) {
        const value = attributes[attribute];
        if (!value) continue;
        const url = new URL(value, `${SITE_URL}${route}`);
        if (url.origin !== SITE_URL) continue;
        const decoded = decodeURIComponent(url.pathname);
        let target = decoded.endsWith('/') ? `${decoded.slice(1)}index.html` : decoded.slice(1);
        if (!(await existsFile(path.join(directory, target)))) {
          const indexTarget = `${target}/index.html`;
          if (await existsFile(path.join(directory, indexTarget))) target = indexTarget;
          else throw new Error(`Broken ${attribute} in ${relative}: ${value}`);
        }
        if (url.hash && documents.has(target) && !documents.get(target).ids.has(decodeURIComponent(url.hash.slice(1)))) {
          throw new Error(`Missing fragment in ${relative}: ${value}`);
        }
      }
    }
  }
  return documents;
}

export async function assertPageContract(directory) {
  for (const route of PAGE_ROUTES) {
    const file = routeFile(route);
    const html = await readFile(path.join(directory, file), 'utf8');
    const canonicals = htmlElements(html).filter(({ tag, attributes }) => tag === 'link' && attributes.rel === 'canonical');
    if (canonicals.length !== 1 || canonicals[0].attributes.href !== `${SITE_URL}${route}`) {
      throw new Error(`Wrong or missing canonical: ${file}`);
    }
    if (!/<h1\b/i.test(html)) throw new Error(`Missing h1: ${file}`);
  }
  const home = htmlElements(await readFile(path.join(directory, 'index.html'), 'utf8'));
  for (const slug of LEGACY_SKILLS) {
    if (!home.some(({ attributes }) => attributes.id === slug)) throw new Error(`Missing legacy homepage anchor: ${slug}`);
  }
  const alias = await readFile(path.join(directory, 'skills/renhua/index.html'), 'utf8');
  if (!htmlElements(alias).some(({ attributes }) => attributes.href === '/skills/no-bs/')) throw new Error('Alias page must link to no-bs');
  if (!(await existsFile(path.join(directory, '404.html')))) throw new Error('Static 404.html is required');
  const sitemap = await readFile(path.join(directory, 'sitemap.xml'), 'utf8');
  const locations = [...sitemap.matchAll(/<loc>([^<]+)<\/loc>/g)].map(match => decodeEntities(match[1]));
  const expected = PAGE_ROUTES.map(route => `${SITE_URL}${route}`);
  if (locations.length !== expected.length || new Set(locations).size !== expected.length || expected.some(location => !locations.includes(location))) {
    throw new Error('Sitemap must contain all canonical pages exactly once');
  }
  const robots = await readFile(path.join(directory, 'robots.txt'), 'utf8');
  if (!robots.includes(`Sitemap: ${SITE_URL}/sitemap.xml`)) throw new Error('robots.txt must advertise sitemap');
}

export async function installationExports(root = workspace) {
  const temporary = await mkdtemp(path.join(os.tmpdir(), 'mrcoder-verify-installation-'));
  try {
    for (const name of ['skills', 'install']) {
      const source = await readFile(path.join(root, `src/data/${name}.ts`), 'utf8');
      const compiled = ts.transpileModule(source, {
        compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
      }).outputText.replace("from './skills'", "from './skills.mjs'");
      await writeFile(path.join(temporary, `${name}.mjs`), compiled);
    }
    return await import(pathToFileURL(path.join(temporary, 'install.mjs')).href);
  } finally { await rm(temporary, { recursive: true, force: true }); }
}

export function expectedSkillsCommand(slug, agent) {
  const selection = slug === undefined ? '' : ` --skill ${['no-bs', 'renhua'].includes(slug) ? 'no-bs renhua' : slug === 'overnight' ? 'overnight unattended' : slug}`;
  const flag = agent === undefined ? '' : ` --agent ${agent === 'claude' ? 'claude-code' : 'codex'}`;
  return `npx skills@latest add ${SKILLS_SOURCE}${selection}${flag}`;
}

export async function assertPluginMetadata(root = workspace) {
  const plugin = JSON.parse(await readFile(path.join(root, '.claude-plugin/plugin.json'), 'utf8'));
  const marketplace = JSON.parse(await readFile(path.join(root, '.claude-plugin/marketplace.json'), 'utf8'));
  if (plugin.name !== PLUGIN_NAME || !/^\d+\.\d+\.\d+$/.test(plugin.version)) throw new Error('Wrong Claude plugin name or version');
  if (marketplace.name !== MARKETPLACE_NAME || !marketplace.owner?.name || marketplace.plugins?.length !== 1) throw new Error('Wrong Claude marketplace identity');
  const entry = marketplace.plugins[0];
  if (entry.name !== PLUGIN_NAME || entry.source !== './' || entry.strict !== true) throw new Error('Claude plugin must use the canonical repository root');
  // Claude discovers root skills/ by default. Redundant paths can load a package twice.
  if ('skills' in plugin || 'skills' in entry || ['hooks', 'mcpServers', 'lspServers'].some(key => key in plugin || key in entry)) throw new Error('Claude plugin must expose only canonical default skills');
  for (const slug of [...LEGACY_SKILLS, 'renhua']) {
    if (!(await existsFile(path.join(root, 'skills', slug, 'SKILL.md')))) throw new Error(`Missing Claude plugin skill: ${slug}`);
  }
  return 12;
}

export async function assertInstallationContract(directory, source) {
  const allowed = new Set(['npx skills update']);
  for (const slug of [undefined, ...LEGACY_SKILLS, 'renhua']) {
    for (const agent of [undefined, 'claude', 'codex']) {
      const expected = expectedSkillsCommand(slug, agent);
      if (source) {
        const actual = slug === undefined ? source.installAllCommand(agent) : source.installCommand(slug, agent);
        if (actual !== expected) throw new Error(`Skills CLI source, dependency or alias mismatch: ${slug ?? 'all'}`);
      }
      allowed.add(expected);
    }
  }
  if (source && (source.skillSource !== SKILLS_SOURCE || source.updateCommand !== 'npx skills update' || source.pluginMarketplaceCommand !== MARKETPLACE_COMMAND || source.pluginInstallCommand !== PLUGIN_COMMAND || source.pluginUpdateCommand !== PLUGIN_UPDATE_COMMAND)) throw new Error('Installation channel source constants differ from publishing contract');
  const pluginAllowed = new Set([MARKETPLACE_COMMAND, PLUGIN_COMMAND, PLUGIN_UPDATE_COMMAND]);
  let skillsCommands = 0;
  let pluginCommands = 0;
  for (const relative of await htmlFiles(directory)) {
    for (const { attributes } of htmlElements(await readFile(path.join(directory, relative), 'utf8'))) {
      if (!attributes['data-commands']) continue;
      const commands = JSON.parse(attributes['data-commands']);
      if (!Array.isArray(commands)) throw new Error(`Invalid command choices in ${relative}`);
      for (const { command } of commands) {
        if (allowed.has(command)) skillsCommands += 1;
        else if (pluginAllowed.has(command)) pluginCommands += 1;
        else throw new Error(`Unknown or incorrect installation command in ${relative}: ${command}`);
      }
    }
  }
  if (!skillsCommands) throw new Error('The built site must expose official skills CLI commands');
  if (!pluginCommands) throw new Error('The built site must expose the Claude plugin channel');
  return { skillsCommands, pluginCommands };
}

export async function verifySite({ root = workspace, directory = path.join(root, 'dist') } = {}) {
  const baseline = JSON.parse(await readFile(fixturePath, 'utf8'));
  await assertPreservedFiles(root, baseline);
  await assertPreservedFiles(directory, baseline);
  if (!(await readFile(path.join(root, '.nojekyll'))).equals(await readFile(path.join(directory, '.nojekyll')))) {
    throw new Error('.nojekyll bytes changed');
  }
  const archiveCount = await assertZipChecksums(directory);
  await assertPageContract(directory);
  const pluginSkills = await assertPluginMetadata(root);
  const commands = await assertInstallationContract(directory, await installationExports(root));
  const documents = await assertInternalLinks(directory);
  return { preservedFiles: Object.keys(baseline).length, archiveCount, pages: documents.size, pluginSkills, ...commands };
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const result = await verifySite();
  console.log(`Verified ${result.preservedFiles} preserved files, ${result.archiveCount} ZIP checksums, ${result.pages} HTML pages and internal links, and ${result.skillsCommands} skills CLI commands plus ${result.pluginCommands} Claude plugin commands (${result.pluginSkills} canonical plugin skills).`);
}
