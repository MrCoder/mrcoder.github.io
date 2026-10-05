import { skills } from './skills';
export type Agent = 'claude' | 'codex';
export const agents: {id: Agent; label: string; destination: string}[] = [
  {id:'claude',label:'Claude Code',destination:'.claude/skills'},
  {id:'codex',label:'Codex',destination:'.agents/skills'},
];
export const skillSource = 'https://github.com/MrCoder/mrcoder.github.io/tree/master';
export const updateCommand = 'npx skills update';
function agentFlag(agent?: Agent): string {
  if (agent === undefined) return '';
  if (agent !== 'claude' && agent !== 'codex') throw new Error(`Unknown agent: ${agent}`);
  return ` --agent ${agent === 'claude' ? 'claude-code' : 'codex'}`;
}
export function installCommand(slug: string, agent?: Agent): string {
  if (![...skills.map(skill => skill.slug), 'renhua'].includes(slug)) throw new Error(`Unknown skill: ${slug}`);
  const selected = slug === 'no-bs' || slug === 'renhua' ? 'no-bs renhua' : slug === 'overnight' ? 'overnight unattended' : slug;
  return `npx skills@latest add ${skillSource} --skill ${selected}${agentFlag(agent)}`;
}
export function installAllCommand(agent?: Agent): string {
  return `npx skills@latest add ${skillSource}${agentFlag(agent)}`;
}
export function shortCommand(slug: string | undefined, agent?: Agent): string {
  return slug === undefined ? installAllCommand(agent) : installCommand(slug, agent);
}
export const pluginMarketplaceCommand = "claude plugin marketplace add 'https://github.com/MrCoder/mrcoder.github.io.git#master'";
export const pluginInstallCommand = 'claude plugin install mrcoder-skills@mrcoder';
export const pluginUpdateCommand = 'claude plugin marketplace update mrcoder\nclaude plugin update mrcoder-skills@mrcoder';
