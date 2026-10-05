export const SITE_URL = 'https://mrcoder.github.io';
export const SKILLS_SOURCE = 'https://github.com/MrCoder/mrcoder.github.io/tree/master';
export const PLUGIN_NAME = 'mrcoder-skills';
export const MARKETPLACE_NAME = 'mrcoder';
export const MARKETPLACE_COMMAND = "claude plugin marketplace add 'https://github.com/MrCoder/mrcoder.github.io.git#master'";
export const PLUGIN_COMMAND = 'claude plugin install mrcoder-skills@mrcoder';
export const PLUGIN_UPDATE_COMMAND = 'claude plugin marketplace update mrcoder\nclaude plugin update mrcoder-skills@mrcoder';

// Compatibility contract captured before the redesign, independent of new UI data.
export const LEGACY_SKILLS = [
  'no-bs', 'search-conversation-history', 'diagram', '1by1', 'unattended',
  'overnight', 'blind-spot-pass', 'deviation-log', 'spot-check', 'merge-quiz', 'pitch-me',
];
export const PAGE_ROUTES = [
  '/', '/install/', '/about/', ...[...LEGACY_SKILLS, 'renhua'].map(slug => `/skills/${slug}/`),
];

export function routeFile(route) {
  return route === '/' ? 'index.html' : `${route.slice(1)}index.html`;
}
