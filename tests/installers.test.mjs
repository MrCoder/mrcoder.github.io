import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { lstat, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { test } from 'node:test';
import ts from 'typescript';

const root = fileURLToPath(new URL('../', import.meta.url));
const skillsCli = process.env.SKILLS_CLI || path.join(root, 'node_modules/skills/bin/cli.mjs');
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
async function temporary(t) {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'field-guide-install-'));
  t.after(() => rm(directory, { recursive: true, force: true }));
  return directory;
}
async function loadInstaller(directory) {
  for (const name of ['skills', 'install']) {
    const source = await readFile(path.join(root, `src/data/${name}.ts`), 'utf8');
    const compiled = ts.transpileModule(source, {
      compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 },
    }).outputText.replace("from './skills'", "from './skills.mjs'");
    await writeFile(path.join(directory, `${name}.mjs`), compiled);
  }
  return import(pathToFileURL(path.join(directory, 'install.mjs')).href);
}
async function packageFiles(directory, prefix = '') {
  const files = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const relative = `${prefix}${entry.name}`;
    if (entry.isDirectory()) files.push(...await packageFiles(path.join(directory, entry.name), `${relative}/`));
    else files.push(relative);
  }
  return files.sort();
}
async function assertCompletePackage(destination, slug) {
  const source = path.join(root, 'skills', slug);
  const installed = path.join(destination, slug);
  const files = await packageFiles(source);
  assert.deepEqual(await packageFiles(installed), files, `${slug}: complete file tree`);
  for (const file of files) {
    const original = path.join(source, file);
    const copy = path.join(installed, file);
    assert.equal(digest(await readFile(copy)), digest(await readFile(original)), `${slug}/${file}: exact bytes`);
    assert.equal((await lstat(copy)).mode & 0o111, (await lstat(original)).mode & 0o111, `${slug}/${file}: executable bits`);
  }
}
test('official CLI commands select master, aliases, dependencies, and supported agents', async t => {
  const directory = await temporary(t);
  const installer = await loadInstaller(directory);
  assert.equal(installer.installAllCommand(), 'npx skills@latest add https://github.com/MrCoder/mrcoder.github.io/tree/master');
  assert.match(installer.installCommand('no-bs','claude'), /--skill no-bs renhua --agent claude-code$/);
  assert.match(installer.installCommand('renhua','codex'), /--skill no-bs renhua --agent codex$/);
  assert.match(installer.installCommand('overnight','codex'), /--skill overnight unattended --agent codex$/);
  assert.equal(installer.updateCommand, 'npx skills update');
  assert.throws(() => installer.installCommand('bad;echo unsafe'), /Unknown skill/);
  assert.throws(() => installer.installAllCommand('unknown'), /Unknown agent/);
});

// The actual published CLI is exercised separately against GitHub in an isolated
// temporary project. Unit checks never fetch packages or install into user folders.
test('all canonical packages have discoverable names and dependency selections exist', async t => {
  const directory = await temporary(t);
  const installer = await loadInstaller(directory);
  for (const slug of await readdir(path.join(root,'skills'))) {
    const text = await readFile(path.join(root,'skills',slug,'SKILL.md'),'utf8');
    assert.match(text, new RegExp(`name: ${slug}\\n`));
    assert.match(installer.installCommand(slug), /npx skills@latest add/);
  }
});

for (const agent of ['claude-code', 'codex']) {
  test(`official CLI ${agent}: copies complete local packages`, async t => {
    await assert.doesNotReject(lstat(skillsCli), 'Official skills CLI is missing; run npm ci to install the pinned test dependency.');
    const directory = await temporary(t);
    await mkdir(path.join(directory, 'project'));
    execFileSync(process.execPath, [skillsCli, 'add', root, '--skill', '*', '--agent', agent, '--copy', '--yes'], {
      cwd: path.join(directory, 'project'), env: {...process.env, DISABLE_TELEMETRY:'1'}, stdio:'pipe',
    });
    const destination = path.join(directory, 'project', agent === 'codex' ? '.agents/skills' : '.claude/skills');
    for (const slug of await readdir(path.join(root,'skills'))) await assertCompletePackage(destination, slug);
  });
}
