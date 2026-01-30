import { defineStore } from "pinia";
import { ref, computed, watch } from "vue";
import { Snackbar } from "@varlet/ui";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { isSqliteAvailable } from "../db/db";
import { deleteAccount, getAccountTokens, listAccounts } from "../db/accountDb";
import { saveGachaRecords, listGachaPulls, deleteInvalidGachaRecords } from "../db/gachaDb";
import type { BannerItem } from "../components/gacha/BannerCard.vue";
import i18n from "../i18n";

const { t } = i18n.global;

type LocaleItem = { itemid?: string; name?: string };
type LocaleNameMaps = { character: Map<string, string>; weapon: Map<string, string> };

// 类型定义
export type SelectOption = { label: string; value: string };

type GachaRecord = {
    name: string;
    item_id: string;
    rarity: number;
    pool_id: string;
    pool_name: string;
    seq_id: string;
    pulled_at: number;
    pool_type: string;
    meta?: any;
};

export type BannerStats = {
    s6: number;
    s5: number;
    s4: number;
    s3: number;
};

type WeaponPool = {
    pool_id: string;
    pool_name: string;
};

type IconGetter = (rec: GachaRecord) => string | undefined;

function normalizeLangTag(lang: string | undefined | null) {
    const raw = (lang || "").trim();
    if (!raw) return "zh-CN";
    const normalized = raw.replace("_", "-");
    const parts = normalized.split("-");
    if (parts.length === 1) return parts[0];
    const language = parts[0].toLowerCase();
    const region = parts[1].toUpperCase();
    return `${language}-${region}`;
}

function isWeapon(rec: { pool_type?: string; pool_name?: string }) {
    const poolType = rec.pool_type || "";
    const poolName = rec.pool_name || "";
    return poolType.includes("Weapon") || poolName.includes("武器");
}

// 辅助函数放在组件外保持 store 简洁
function sum(stats: BannerStats) {
    return stats.s6 + stats.s5 + stats.s4 + stats.s3;
}

function getHistory(records: GachaRecord[], limit = 50, iconGetter?: IconGetter) {
    const sorted = [...records].reverse();
    const history: Array<{ name: string; count: number; rarity: 6 | 5 | 4 | 3; icon?: string }> = [];
    let pity = 0;
    for (const rec of sorted) {
        pity++;
        if (rec.rarity === 6) {
            const icon = iconGetter?.(rec);
            history.push({ name: rec.name, count: pity, rarity: 6, icon });
            pity = 0;
        }
    }
    return history.reverse().slice(0, limit);
}

function buildStats(records: GachaRecord[]) {
    let s6 = 0;
    let s5 = 0;
    let s4 = 0;
    // 不统计三星/二星
    let pullsSinceLast6 = 0;
    let min6 = Infinity;
    let max6 = 0;
    const total = records.length;

    let guarantee = 0;
    let foundFirst6 = false;

    // 记录按从新到旧遍历
    for (const rec of records) {
        pullsSinceLast6 += 1;
        if (rec.rarity === 6) {
            s6 += 1;

            if (!foundFirst6) {
                // 最新的首个六星
                guarantee = pullsSinceLast6 - 1;
                foundFirst6 = true;
            } else {
                // 后续六星（完成一轮保底）
                min6 = Math.min(min6, pullsSinceLast6);
                max6 = Math.max(max6, pullsSinceLast6);
            }

            pullsSinceLast6 = 0;
        } else if (rec.rarity === 5) {
            s5 += 1;
        } else if (rec.rarity === 4) {
            s4 += 1;
        }
        // 三星以下不计入
    }

    // 若从未出过六星，保底计数为当前累计
    if (!foundFirst6) {
        guarantee = pullsSinceLast6;
    } else {
        // 处理最早的那次六星：它的代价是“之后的抽数”+自身
        // pullsSinceLast6 在遇到六星时会清零，此处统计的是最早那次六星前的抽数
        const lastCost = pullsSinceLast6 + 1;
        min6 = Math.min(min6, lastCost);
        max6 = Math.max(max6, lastCost);
    }

    const avg6 = s6 > 0 ? Math.round(total / s6) : 0;

    return {
        stats: { s6, s5, s4, s3: 0 },
        guarantee,
        avg6,
        min6: min6 === Infinity ? 0 : min6,
        max6,
    };
}

function formatRange(records: GachaRecord[]) {
    if (!records.length) return "";
    const times = records
        .map((r) => {
            let t = Number(r.pulled_at);
            if (t > 0 && t < 10000000000) t *= 1000;
            return t;
        })
        .filter((n) => Number.isFinite(n) && n > 0)
        .sort((a, b) => a - b);
    if (!times.length) return "";
    const fmt = (n: number) => {
        const d = new Date(n);
        const y = d.getFullYear();
        const m = String(d.getMonth() + 1).padStart(2, "0");
        const day = String(d.getDate()).padStart(2, "0");
        return `${y}.${m}.${day}`;
    };
    return times.length === 1 ? fmt(times[0]) : `${fmt(times[0])} - ${fmt(times[times.length - 1])}`;
}

function generateBanners(charBeginner: GachaRecord[], charSpecial: GachaRecord[], charStandard: GachaRecord[], weaponPools: Map<string, GachaRecord[]>, iconGetter?: IconGetter): BannerItem[] {
    const nextBanners: BannerItem[] = [];

    if (charBeginner.length) {
        const stat = buildStats(charBeginner);
        nextBanners.push({
            id: "char-beginner",
            title: t("gacha.banner.beginnerTitle"),
            range: formatRange(charBeginner),
            topLabel: t("gacha.banner.topLabel"),
            stats: stat.stats,
            guarantee: stat.guarantee,
            avg6: stat.avg6,
            min6: stat.min6,
            max6: stat.max6,
            top: getHistory(charBeginner, 50, iconGetter),
        });
    }

    if (charSpecial.length) {
        const stat = buildStats(charSpecial);
        nextBanners.push({
            id: "char-special",
            title: t("gacha.banner.specialTitle"),
            range: formatRange(charSpecial),
            topLabel: t("gacha.banner.topLabel"),
            stats: stat.stats,
            guarantee: stat.guarantee,
            avg6: stat.avg6,
            min6: stat.min6,
            max6: stat.max6,
            top: getHistory(charSpecial, 50, iconGetter),
        });
    }

    if (charStandard.length) {
        const stat = buildStats(charStandard);
        nextBanners.push({
            id: "char-standard",
            title: t("gacha.banner.standardTitle"),
            range: formatRange(charStandard),
            topLabel: t("gacha.banner.topLabel"),
            stats: stat.stats,
            guarantee: stat.guarantee,
            avg6: stat.avg6,
            min6: stat.min6,
            max6: stat.max6,
            top: getHistory(charStandard, 50, iconGetter),
        });
    }

    // 武器卡池
    for (const [poolId, records] of weaponPools.entries()) {
        if (!records.length) continue;
        const stat = buildStats(records);
        const poolName = records[0]?.pool_name || t("gacha.banner.weaponUnknown");
        nextBanners.push({
            id: `weapon-${poolId}`,
            title: t("gacha.banner.weaponTitle", { name: poolName }),
            range: formatRange(records),
            topLabel: t("gacha.banner.topLabel"),
            stats: stat.stats,
            guarantee: stat.guarantee,
            avg6: stat.avg6,
            min6: stat.min6,
            max6: stat.max6,
            top: getHistory(records, 50, iconGetter),
        });
    }

    return nextBanners;
}


export const useGachaStore = defineStore("gacha", () => {
    const uid = ref("");
    const uidOptions = ref<SelectOption[]>([]);
    const loading = ref(false);
    const banners = ref<BannerItem[]>([]);
    const opened = ref<(string | number)[]>([]);
    const metadataDir = ref<string | null>(null);

    // 计算属性
    const accountsList = ref<any[]>([]);

    // 计算属性
    const currentNickname = computed(() => {
        if (!uid.value) return "";
        const acc = accountsList.value.find(a => a.uid === uid.value);
        return acc?.nickName || "";
    });

    const bannerSummary = computed(() =>
        banners.value.map((b) => ({
            ...b,
            total: sum(b.stats),
        }))
    );

    let metadataDirPromise: Promise<void> | null = null;
    async function ensureMetadataDir() {
        if (metadataDirPromise) return metadataDirPromise;
        metadataDirPromise = (async () => {
            try {
                const status = await invoke<{ path: string }>("check_metadata");
                const baseDirRaw = status?.path || "";
                if (!baseDirRaw) return;
                metadataDir.value = baseDirRaw.replace(/^\\\\\?\\/, "").replace(/\\/g, "/");
            } catch (e) {
                console.error("load metadata dir failed", e);
            }
        })();
        return metadataDirPromise;
    }

    const localeMapsCache = new Map<string, Promise<LocaleNameMaps>>();
    async function fetchLocaleList(baseDir: string, lang: string, fileNames: string[]) {
        const tryDirs = [
            `${baseDir}/locale/${lang}`,
        ];
        for (const dir of tryDirs) {
            for (const fileName of fileNames) {
                const fullPath = `${dir}/${fileName}`;
                try {
                    const url = convertFileSrc(fullPath);
                    const res = await fetch(url);
                    if (!res.ok) continue;
                    const json = (await res.json()) as unknown;
                    if (Array.isArray(json)) return json as LocaleItem[];
                } catch (e) {
                    // 出错则继续尝试下一个候选文件
                }
            }
        }
        return null;
    }

    async function ensureLocaleMaps(): Promise<LocaleNameMaps> {
        await ensureMetadataDir();
        const baseDir = metadataDir.value;
        const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);
        if (!baseDir || !isTauri) {
            return { character: new Map(), weapon: new Map() };
        }

        const lang = normalizeLangTag(i18n.global.locale.value);
        const cacheKey = `${baseDir}::${lang}`;
        if (localeMapsCache.has(cacheKey)) return localeMapsCache.get(cacheKey)!;

        const promise = (async () => {
            const fallbackLang = "zh-CN";
            const characterNames =
                (await fetchLocaleList(baseDir, lang, ["character.json", "charater.json"])) ||
                (lang !== fallbackLang ? await fetchLocaleList(baseDir, fallbackLang, ["character.json", "charater.json"]) : null) ||
                [];
            const weaponNames =
                (await fetchLocaleList(baseDir, lang, ["weapon.json"])) ||
                (lang !== fallbackLang ? await fetchLocaleList(baseDir, fallbackLang, ["weapon.json"]) : null) ||
                [];

            const character = new Map<string, string>();
            const weapon = new Map<string, string>();
            for (const item of characterNames) {
                if (item?.itemid && item?.name) character.set(item.itemid, item.name);
            }
            for (const item of weaponNames) {
                if (item?.itemid && item?.name) weapon.set(item.itemid, item.name);
            }
            return { character, weapon };
        })();

        localeMapsCache.set(cacheKey, promise);
        return promise;
    }

    // 行为
    async function loadFromDb(targetUid: string) {
        if (!isSqliteAvailable() || !targetUid) return;
        try {
            await ensureMetadataDir();
            const baseDir = metadataDir.value;
            const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in (window as any);
            const iconCache = new Map<string, string | undefined>();
            const localeMaps = await ensureLocaleMaps();
            const iconGetter: IconGetter | undefined = baseDir && isTauri
                ? (rec) => {
                    if (!rec.item_id) return undefined;
                    const category = isWeapon(rec) ? "weapon" : "character";
                    const key = `${category}:${rec.item_id}`;
                    if (iconCache.has(key)) return iconCache.get(key);
                    const fullPath = `${baseDir}/images/icon/${category}/${rec.item_id}.png`;
                    try {
                        const url = convertFileSrc(fullPath);
                        iconCache.set(key, url);
                        return url;
                    } catch (e) {
                        console.error("convertFileSrc failed", e);
                        iconCache.set(key, undefined);
                        return undefined;
                    }
                }
                : undefined;

            const pulls = await listGachaPulls(targetUid, 10000);
            const records: GachaRecord[] = pulls.map((p) => ({
                name: (() => {
                    const id = p.itemId || "";
                    if (!id) return p.itemName;
                    const localized = isWeapon({ pool_type: p.poolType || "", pool_name: p.bannerName || "" })
                        ? localeMaps.weapon.get(id)
                        : localeMaps.character.get(id);
                    return localized || p.itemName || id;
                })(),
                item_id: p.itemId || "",
                rarity: p.rarity,
                pool_id: p.bannerId,
                pool_name: p.bannerName,
                seq_id: p.seqId || "",
                pulled_at: p.pulledAt,
                pool_type: p.poolType || "",
            }));

            const charSpecial: GachaRecord[] = [];
            const charStandard: GachaRecord[] = [];
            const charBeginner: GachaRecord[] = [];


            const weaponPoolsMap = new Map<string, GachaRecord[]>();

            for (const r of records) {
                const type = r.pool_type;
                // Hypergryph 返回的官方枚举
                if (type === "E_CharacterGachaPoolType_Special") charSpecial.push(r);
                else if (type === "E_CharacterGachaPoolType_Standard") charStandard.push(r);
                else if (type === "E_CharacterGachaPoolType_Beginner") charBeginner.push(r);
                else if (type === "E_CharacterGachaPoolType_Weapon") {
                    const pid = r.pool_id || "other";
                    if (!weaponPoolsMap.has(pid)) weaponPoolsMap.set(pid, []);
                    weaponPoolsMap.get(pid)!.push(r);
                }
                else {
                    // 旧数据缺少 poolType 时根据名称兜底分类（无元数据可参照）

                    if (type.includes("Special")) charSpecial.push(r);
                    else if (type.includes("Standard")) charStandard.push(r);
                    else if (type.includes("Beginner")) charBeginner.push(r);
                    else if (type.includes("Weapon")) {
                        const pid = r.pool_id || "legacy-weapon";
                        if (!weaponPoolsMap.has(pid)) weaponPoolsMap.set(pid, []);
                        weaponPoolsMap.get(pid)!.push(r);
                    }
                    else if (r.pool_name.includes("武器")) {
                        const pid = r.pool_id || "legacy-weapon";
                        if (!weaponPoolsMap.has(pid)) weaponPoolsMap.set(pid, []);
                        weaponPoolsMap.get(pid)!.push(r);
                    }
                    else if (r.pool_name.includes("常规") || r.pool_name.includes("标准")) charStandard.push(r);
                    else if (r.pool_name.includes("启程")) charBeginner.push(r);
                    else charSpecial.push(r);
                }
            }

            // 使用 seq_id 倒序，pulled_at 可能重复，seq_id 更稳定
            const sortDesc = (a: GachaRecord, b: GachaRecord) => {
                // seq_id 比较：长度优先，其次字典序
                if (a.seq_id.length !== b.seq_id.length) {
                    return b.seq_id.length - a.seq_id.length;
                }
                return b.seq_id.localeCompare(a.seq_id);
            };
            charSpecial.sort(sortDesc);
            charStandard.sort(sortDesc);
            charBeginner.sort(sortDesc);
            for (const records of weaponPoolsMap.values()) {
                records.sort(sortDesc);
            }

            const nextBanners = generateBanners(charBeginner, charSpecial, charStandard, weaponPoolsMap, iconGetter);
            if (nextBanners.length) {
                banners.value = nextBanners;
                // 初始时全部展开，保留用户已展开状态
                if (opened.value.length === 0) {
                    opened.value = nextBanners.map((b) => b.id);
                }
            } else {
                banners.value = [];
            }
        } catch (e) {
            console.error("Local load failed", e);
        }
    }

    watch(uid, (newUid) => {
        if (newUid) {
            void loadFromDb(newUid);
        } else {
            banners.value = [];
        }
    });

    watch(() => i18n.global.locale.value, () => {
        localeMapsCache.clear();
        if (uid.value) void loadFromDb(uid.value);
    });

    async function reloadAccounts(preferUid?: string) {
        if (!isSqliteAvailable()) {
            uidOptions.value = [];
            uid.value = "";
            return;
        }

        const accounts = await listAccounts();
        accountsList.value = accounts;

        // 优先展示游戏内角色 ID，缺失时退回 UID
        uidOptions.value = accounts.map((a) => {
            const label = a.roleId ? `${a.roleId}` : a.uid;
            return { label, value: a.uid };
        });

        const nextUid =
            preferUid && uidOptions.value.some((o) => o.value === preferUid)
                ? preferUid
                : uidOptions.value.some((o) => o.value === uid.value)
                    ? uid.value
                    : uidOptions.value[0]?.value ?? "";

        uid.value = nextUid;
        // 修改 uid 会触发 watch(uid) 重新加载记录
    }

    async function refreshGacha(mode: "incremental" | "full" = "incremental") {
        if (!uid.value) {
            Snackbar.warning(t("gacha.messages.selectAccount"));
            return;
        }
        if (!isSqliteAvailable()) {
            Snackbar.warning(t("gacha.messages.tauriOnly"));
            return;
        }
        loading.value = true;
        try {
            const account = await getAccountTokens(uid.value);
            if (!account?.oauthToken) throw new Error(t("gacha.messages.missingToken"));

            // 刷新 u8Token（有效期短）
            const token = await invoke<string>("hg_u8_token_by_uid", {
                uid: account.uid,
                oauthToken: account.oauthToken,
            });

            const serverId = "1";

            // 增量模式下记录每个卡池的最新 seq_id
            const lastSeqMap = new Map<string, string>();
            if (mode === "incremental") {
                // 简单取最近 1000 条记录推断最新 seq_id
                const pulls = await listGachaPulls(uid.value, 1000); // 取最近 1000 条足够
                for (const p of pulls) {
                    const key = p.poolType === "E_CharacterGachaPoolType_Weapon" ? p.bannerId : p.poolType;
                    if (key && !lastSeqMap.has(key)) {
                        lastSeqMap.set(key, p.seqId || "");
                    }
                }
            }

            console.log(`[gacha] refreshing mode=${mode}, lastSeqMap size=${lastSeqMap.size}`);

            // 调用 Tauri 后端避免浏览器 CORS
            const [charSpecial, charStandard, charBeginner] = await Promise.all([
                invoke<GachaRecord[]>("hg_fetch_char_records", {
                    token, serverId, poolType: "E_CharacterGachaPoolType_Special",
                    lastSeqIdStop: lastSeqMap.get("E_CharacterGachaPoolType_Special")
                }).catch((err: unknown) => { console.error("[gacha] charSpecial error:", err); return []; }),
                invoke<GachaRecord[]>("hg_fetch_char_records", {
                    token, serverId, poolType: "E_CharacterGachaPoolType_Standard",
                    lastSeqIdStop: lastSeqMap.get("E_CharacterGachaPoolType_Standard")
                }).catch((err: unknown) => { console.error("[gacha] charStandard error:", err); return []; }),
                invoke<GachaRecord[]>("hg_fetch_char_records", {
                    token, serverId, poolType: "E_CharacterGachaPoolType_Beginner",
                    lastSeqIdStop: lastSeqMap.get("E_CharacterGachaPoolType_Beginner")
                }).catch((err: unknown) => { console.error("[gacha] charBeginner error:", err); return []; }),
            ]);

            const pools = await invoke<WeaponPool[]>("hg_fetch_weapon_pools", { token, serverId })
                .catch((err: unknown) => { console.error("[gacha] weaponPools error:", err); return []; });

            const weaponRecordsPromise = pools.map(pool =>
                invoke<GachaRecord[]>("hg_fetch_weapon_records", {
                    token, serverId, poolId: pool.pool_id,
                    lastSeqIdStop: lastSeqMap.get(pool.pool_id)
                }).catch((err: unknown) => {
                    console.error(`[gacha] weaponRecords error for pool ${pool.pool_name}:`, err);
                    return [];
                })
            );

            const weaponRecordsResults = await Promise.all(weaponRecordsPromise);
            const weaponRecords = weaponRecordsResults.flat();

            const allFetched = [...charBeginner, ...charSpecial, ...charStandard, ...weaponRecords];
            if (allFetched.length > 0 && isSqliteAvailable()) {
                try {
                    if (mode === "full") {
                        await deleteInvalidGachaRecords(uid.value);
                    }
                    await saveGachaRecords(uid.value, allFetched);
                    Snackbar.success(mode === "incremental"
                        ? t("gacha.messages.syncIncremental", { count: allFetched.length })
                        : t("gacha.messages.syncFull"));
                    await loadFromDb(uid.value);
                } catch (e) {
                    console.error(e);
                    Snackbar.error(t("gacha.messages.saveFailed"));
                }
            } else {
                Snackbar.info(t("gacha.messages.noNewRecords"));
            }
        } catch (err) {
            Snackbar.error((err as Error)?.message ?? String(err));
        } finally {
            loading.value = false;
        }
    }

    async function deleteCurrentAccount() {
        if (!uid.value) {
            Snackbar.warning(t("gacha.messages.selectDelete"));
            return;
        }
        const sure = window.confirm(t("gacha.messages.confirmDelete", { uid: uid.value }));
        if (!sure) return;
        try {
            await deleteAccount(uid.value);
            Snackbar.success(t("gacha.messages.deleteSuccess"));
            await reloadAccounts();
        } catch (err) {
            Snackbar.error((err as Error)?.message ?? String(err));
        }
    }

    return {
        uid,
        uidOptions,
        loading,
        banners,
        opened,
        bannerSummary,
        reloadAccounts, // 提供给初始化或手动刷新调用
        refreshGacha,
        deleteCurrentAccount,
        currentNickname,
    };
});
