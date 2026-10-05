import { execFileSync } from 'node:child_process';
import { copyFile, lstat, mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { PAGE_ROUTES, SITE_URL } from './site-contract.mjs';

const workspace = fileURLToPath(new URL('../', import.meta.url));

export function trackedPublishPaths(root = workspace) {
  return execFileSync('git', ['ls-files', '-z', '--', 'skills', 'downloads', 'assets', '.nojekyll'], {
    cwd: root, encoding: 'utf8',
  }).split('\0').filter(Boolean);
}

export async function publishStatic({ root = workspace, destination = path.join(root, 'dist') } = {}) {
  // Never recurse over these directories: untracked caches and scratch files are not public.
  const files = trackedPublishPaths(root);
  if (!files.includes('.nojekyll')) throw new Error('Tracked .nojekyll is required for GitHub Pages');
  for (const relative of files) {
    const source = path.join(root, relative);
    if (!(await lstat(source)).isFile()) throw new Error(`Publish source must be a regular file: ${relative}`);
    const target = path.join(destination, relative);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(source, target);
  }
  const locations = PAGE_ROUTES.map(route => `  <url><loc>${SITE_URL}${route}</loc></url>`).join('\n');
  await writeFile(path.join(destination, 'sitemap.xml'),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${locations}\n</urlset>\n`);
  await writeFile(path.join(destination, 'robots.txt'), `User-agent: *\nAllow: /\nSitemap: ${SITE_URL}/sitemap.xml\n`);
  return files;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  const files = await publishStatic();
  console.log(`Published ${files.length} tracked static files; generated sitemap and robots.txt.`);
}
