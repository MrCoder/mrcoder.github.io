import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { publishStatic, trackedPublishPaths } from '../scripts/publish-static.mjs';
import { assertInstallationContract, assertPluginMetadata, expectedSkillsCommand, installationExports, assertInternalLinks, assertPageContract, assertPreservedFiles, assertZipChecksums } from '../scripts/verify-site.mjs';
import { MARKETPLACE_COMMAND, PLUGIN_COMMAND, LEGACY_SKILLS, PAGE_ROUTES, SITE_URL, routeFile } from '../scripts/site-contract.mjs';

const hash = value => createHash('sha256').update(value).digest('hex');
async function temporary(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'field-guide-test-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}
async function put(directory, relative, bytes) {
  const target = path.join(directory, relative);
  await mkdir(path.dirname(target), { recursive: true });
  await writeFile(target, bytes);
}

test('publishing copies tracked package bytes and excludes untracked scratch files', async t => {
  const root = await temporary(t);
  execFileSync('git', ['init', '--quiet'], { cwd: root });
  const bytes = Buffer.from([0, 10, 255, 128]);
  await put(root, 'skills/example/SKILL.md', '# Example\n');
  await put(root, 'skills/example/bin/tool', bytes);
  await put(root, 'downloads/example.zip', bytes);
  await put(root, 'assets/analytics.js', 'window.test = true;\n');
  await put(root, '.nojekyll', '');
  execFileSync('git', ['add', 'skills', 'downloads', 'assets', '.nojekyll'], { cwd: root });
  await put(root, 'skills/example/scratch.txt', 'private scratch');
  assert.equal(trackedPublishPaths(root).length, 5);
  const files = await publishStatic({ root });
  assert.equal(files.length, 5);
  assert.deepEqual(await readFile(path.join(root, 'dist/skills/example/bin/tool')), bytes);
  await assert.rejects(readFile(path.join(root, 'dist/skills/example/scratch.txt')), { code: 'ENOENT' });
  assert.match(await readFile(path.join(root, 'dist/robots.txt'), 'utf8'), /Sitemap: https:\/\/mrcoder.github.io\/sitemap.xml/);
});

test('immutable release fixture covers every existing package/download/asset', async () => {
  const baseline = JSON.parse(await readFile(new URL('./fixtures/published-files.json', import.meta.url), 'utf8'));
  const paths = (await readFile(new URL('./fixtures/published-paths.txt', import.meta.url), 'utf8')).trim().split('\n');
  assert.equal(paths.length, 66);
  assert.deepEqual(Object.keys(baseline), paths);
  const tracked = trackedPublishPaths().filter(file => file !== '.nojekyll');
  assert.deepEqual(tracked, paths);
  await assertPreservedFiles(new URL('../', import.meta.url).pathname, baseline);
});

test('preservation gate rejects a changed raw instruction even if source and dist agree', async t => {
  const directory = await temporary(t);
  await put(directory, 'skills/example/SKILL.md', 'changed instruction');
  await assert.rejects(assertPreservedFiles(directory, { 'skills/example/SKILL.md': hash('original instruction') }), /Published bytes changed/);
});

test('ZIP gate rejects stale checksum and wrong archive filename', async t => {
  const directory = await temporary(t);
  for (const name of ['one.zip', 'two.zip']) {
    await put(directory, `downloads/${name}`, `bytes of ${name}`);
    await put(directory, `downloads/${name}.sha256`, `${hash(`bytes of ${name}`)}  ${name}\n`);
  }
  assert.equal(await assertZipChecksums(directory), 2);
  await put(directory, 'downloads/one.zip', 'damaged ZIP');
  await assert.rejects(assertZipChecksums(directory), /ZIP checksum mismatch/);
  await put(directory, 'downloads/one.zip.sha256', `${hash('damaged ZIP')}  wrong.zip\n`);
  await assert.rejects(assertZipChecksums(directory), /Invalid checksum file/);
});

test('internal-link gate resolves page routes, raw files and cross-page fragments', async t => {
  const directory = await temporary(t);
  await put(directory, 'index.html', '<main id="catalogue"><a href="/skills/example/#install">Install</a><a href="/skills/example/SKILL.md">Raw</a><script>const ignored = "<a href=\"/bad/\">";</script></main>');
  await put(directory, 'skills/example/index.html', '<h1>Example</h1><section id="install"></section><a href="/#catalogue">Home</a>');
  await put(directory, 'skills/example/SKILL.md', '# Example');
  assert.equal((await assertInternalLinks(directory)).size, 2);
  await put(directory, 'skills/example/index.html', '<h1>Example</h1><a href="/#missing">Broken</a>');
  await assert.rejects(assertInternalLinks(directory), /Missing fragment/);
  await put(directory, 'skills/example/index.html', '<h1 id="install">Example</h1><a href="/absent/">Broken</a>');
  await assert.rejects(assertInternalLinks(directory), /Broken href/);
});

test('page gate rejects lost homepage anchors and wrong canonicals', async t => {
  const directory = await temporary(t);
  for (const route of PAGE_ROUTES) {
    const extra = route === '/' ? LEGACY_SKILLS.map(slug => `<div id="${slug}"></div>`).join('') : route === '/skills/renhua/' ? '<a href="/skills/no-bs/">no-bs</a>' : '';
    await put(directory, routeFile(route), `<link rel="canonical" href="${SITE_URL}${route}"><h1>Page</h1>${extra}`);
  }
  await put(directory, '404.html', '<h1>Not found</h1>');
  await put(directory, 'sitemap.xml', `<urlset>${PAGE_ROUTES.map(route => `<url><loc>${SITE_URL}${route}</loc></url>`).join('')}</urlset>`);
  await put(directory, 'robots.txt', `Sitemap: ${SITE_URL}/sitemap.xml\n`);
  await assertPageContract(directory);
  await put(directory, 'index.html', `<link rel="canonical" href="${SITE_URL}/"><h1>Home</h1>`);
  await assert.rejects(assertPageContract(directory), /Missing legacy homepage anchor/);
  await put(directory, 'index.html', '<link rel="canonical" href="https://example.com/"><h1>Home</h1>');
  await assert.rejects(assertPageContract(directory), /Wrong or missing canonical/);
});

test('installation gate rejects stale endpoints, unpinned sources and missing companion skills', async t => {
  const directory = await temporary(t);
  const page = commands => `<div data-commands='${JSON.stringify(commands.map(command => ({ command }))).replaceAll("'", '&#39;')}'></div>`;
  await put(directory, 'index.html', page([expectedSkillsCommand('no-bs', 'codex'), MARKETPLACE_COMMAND, PLUGIN_COMMAND]));
  assert.deepEqual(await assertInstallationContract(directory), { skillsCommands: 1, pluginCommands: 2 });
  await put(directory, 'index.html', page([expectedSkillsCommand('no-bs', 'codex').replace('no-bs renhua', 'no-bs'), PLUGIN_COMMAND]));
  await assert.rejects(assertInstallationContract(directory), /Unknown or incorrect/);
  await put(directory, 'index.html', page([expectedSkillsCommand('diagram').replace('/tree/master', ''), PLUGIN_COMMAND]));
  await assert.rejects(assertInstallationContract(directory), /Unknown or incorrect/);
  await put(directory, 'index.html', page(['curl -fsSL https://mrcoder.github.io/install.sh | bash', PLUGIN_COMMAND]));
  await assert.rejects(assertInstallationContract(directory), /Unknown or incorrect/);
  await put(directory, 'index.html', page(['npx skills update && curl https://example.com', PLUGIN_COMMAND]));
  await assert.rejects(assertInstallationContract(directory), /Unknown or incorrect/);
  await put(directory, 'index.html', page(['npx skills update', PLUGIN_COMMAND]));
  assert.deepEqual(await assertInstallationContract(directory), { skillsCommands: 1, pluginCommands: 1 });
  await put(directory, 'index.html', page([expectedSkillsCommand('diagram')]));
  await assert.rejects(assertInstallationContract(directory), /Claude plugin channel/);
});

test('current command generator preserves branch, alias and shared-core dependency contracts', async t => {
  const directory = await temporary(t);
  const source = await installationExports();
  const commands = [source.installAllCommand(), source.installCommand('renhua', 'claude'), source.installCommand('overnight', 'codex'), PLUGIN_COMMAND];
  await put(directory, 'index.html', `<div data-commands='${JSON.stringify(commands.map(command => ({ command })))}'></div>`);
  await assertInstallationContract(directory, source);
  await assert.rejects(assertInstallationContract(directory, { ...source, installCommand: () => 'wrong' }), /source, dependency or alias mismatch/);
});

test('plugin metadata loads canonical packages once and rejects a redirected source', async t => {
  assert.equal(await assertPluginMetadata(), 12);
  const directory = await temporary(t);
  const root = new URL('../', import.meta.url).pathname;
  for (const file of ['.claude-plugin/plugin.json', '.claude-plugin/marketplace.json']) await put(directory, file, await readFile(path.join(root, file)));
  for (const slug of [...LEGACY_SKILLS, 'renhua']) await put(directory, `skills/${slug}/SKILL.md`, '# Skill');
  assert.equal(await assertPluginMetadata(directory), 12);
  const manifest = JSON.parse(await readFile(path.join(directory, '.claude-plugin/marketplace.json')));
  manifest.plugins[0].source = './duplicate-packages';
  await put(directory, '.claude-plugin/marketplace.json', JSON.stringify(manifest));
  await assert.rejects(assertPluginMetadata(directory), /canonical repository root/);
  manifest.plugins[0].source = './'; manifest.plugins[0].skills = ['./skills'];
  await put(directory, '.claude-plugin/marketplace.json', JSON.stringify(manifest));
  await assert.rejects(assertPluginMetadata(directory), /canonical default skills/);
});
