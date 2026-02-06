export function channelLabelKey(channelId: number | null | undefined, serverId: string | null | undefined): string | null {
  if (channelId === 1) return "gacha.channels.cnOfficial";
  if (channelId === 2) return "gacha.channels.cnBilibili";
  if (channelId === 6 && serverId === "2") return "gacha.channels.globalAsia";
  if (channelId === 6 && serverId === "3") return "gacha.channels.globalUsEu";
  return null;
}
