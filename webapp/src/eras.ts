// The game data's two eras: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era.
export type Era = 'global' | 'gm';
/** each era's name, as the era switch's buttons say it */
export const ERA_NAMES: Record<Era, string> = { global: 'Global', gm: 'JP, Grand Masters' };
