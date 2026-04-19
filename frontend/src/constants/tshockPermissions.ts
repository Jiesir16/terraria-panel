/**
 * TShock 5.x 完整权限节点树
 * 来源: https://github.com/Pryaxis/TShock/blob/general-devel/TShockAPI/Permissions.cs
 */

export interface PermissionNode {
  key: string        // 权限节点字符串，如 "tshock.admin.kick"
  label: string      // 显示名称
  children?: PermissionNode[]
}

export const TSHOCK_PERMISSION_TREE: PermissionNode[] = [
  {
    key: 'tshock.account',
    label: '账号管理 (account)',
    children: [
      { key: 'tshock.account.register', label: '注册账号' },
      { key: 'tshock.account.login', label: '登录' },
      { key: 'tshock.account.logout', label: '登出' },
      { key: 'tshock.account.changepassword', label: '修改密码' },
    ]
  },
  {
    key: 'tshock.admin',
    label: '管理操作 (admin)',
    children: [
      { key: 'tshock.admin.kick', label: '踢出玩家' },
      { key: 'tshock.admin.ban', label: '封禁玩家' },
      { key: 'tshock.admin.mute', label: '禁言玩家' },
      { key: 'tshock.admin.broadcast', label: '全服广播' },
      { key: 'tshock.admin.warp', label: '传送点管理' },
      { key: 'tshock.admin.group', label: '组管理' },
      { key: 'tshock.admin.region', label: '区域管理' },
      { key: 'tshock.admin.itemban', label: '物品封禁管理' },
      { key: 'tshock.admin.projectileban', label: '弹幕封禁管理' },
      { key: 'tshock.admin.tileban', label: '方块封禁管理' },
      { key: 'tshock.admin.antibuild', label: '建造保护状态' },
      { key: 'tshock.admin.nokick', label: '免踢保护' },
      { key: 'tshock.admin.viewlogs', label: '查看日志' },
      { key: 'tshock.admin.seeplayerids', label: '查看玩家ID' },
      { key: 'tshock.admin.savessi', label: '保存角色状态' },
      { key: 'tshock.admin.tempgroup', label: '临时提权' },
      { key: 'tshock.admin.userinfo', label: '查看用户信息' },
    ]
  },
  {
    key: 'tshock.buff',
    label: '增益效果 (buff)',
    children: [
      { key: 'tshock.buff.self', label: '给自己加Buff' },
      { key: 'tshock.buff.others', label: '给他人加Buff' },
    ]
  },
  {
    key: 'tshock.cfg',
    label: '服务器配置 (cfg)',
    children: [
      { key: 'tshock.cfg.maintenance', label: '服务器维护/更新' },
      { key: 'tshock.cfg.whitelist', label: '白名单管理' },
      { key: 'tshock.cfg.password', label: '修改服务器密码' },
      { key: 'tshock.cfg.reload', label: '重载配置文件' },
      { key: 'tshock.cfg.createdumps', label: '创建参考文件' },
    ]
  },
  {
    key: 'tshock.ignore',
    label: '绕过检测 (ignore)',
    children: [
      { key: 'tshock.ignore.removetile', label: '绕过拆除检测' },
      { key: 'tshock.ignore.placetile', label: '绕过放置检测' },
      { key: 'tshock.ignore.liquid', label: '绕过液体检测' },
      { key: 'tshock.ignore.projectile', label: '绕过弹幕检测' },
      { key: 'tshock.ignore.paint', label: '绕过喷漆检测' },
      { key: 'tshock.ignore.itemstack', label: '绕过堆叠检测' },
      { key: 'tshock.ignore.damage', label: '绕过伤害上限' },
      { key: 'tshock.ignore.npcbuff', label: '绕过NPC增益检测' },
      { key: 'tshock.ignore.ssc', label: '绕过SSC检查' },
      { key: 'tshock.ignore.sendtilesquare', label: '客户端世界编辑' },
      { key: 'tshock.ignore.dropbanneditem', label: '丢弃封禁物品' },
      { key: 'tshock.ignore.hp', label: '绕过异常HP检测' },
      { key: 'tshock.ignore.mp', label: '绕过异常MP检测' },
    ]
  },
  {
    key: 'tshock.item',
    label: '物品管理 (item)',
    children: [
      { key: 'tshock.item.give', label: '给予物品' },
      { key: 'tshock.item.spawn', label: '生成物品' },
      { key: 'tshock.item.usebanned', label: '使用封禁物品' },
    ]
  },
  {
    key: 'tshock.npc',
    label: 'NPC 管理 (npc)',
    children: [
      { key: 'tshock.npc.spawnmob', label: '生成 NPC' },
      { key: 'tshock.npc.spawnboss', label: '生成 Boss' },
      { key: 'tshock.npc.summonboss', label: '使用物品召唤Boss' },
      { key: 'tshock.npc.butcher', label: '清除所有敌怪' },
      { key: 'tshock.npc.invade', label: '发起入侵事件' },
      { key: 'tshock.npc.startinvasion', label: '物品触发入侵' },
      { key: 'tshock.npc.startdd2', label: '启动撒旦军队事件' },
      { key: 'tshock.npc.maxspawns', label: '修改最大生成数' },
      { key: 'tshock.npc.spawnrate', label: '修改生成速率' },
      { key: 'tshock.npc.spawnpets', label: '生成宠物' },
      { key: 'tshock.npc.hurttown', label: '伤害城镇NPC' },
      { key: 'tshock.npc.rename', label: '重命名NPC' },
      { key: 'tshock.npc.clearanglerquests', label: '清除钓鱼任务' },
    ]
  },
  {
    key: 'tshock.tp',
    label: '传送 (tp)',
    children: [
      { key: 'tshock.tp.self', label: '传送到他人' },
      { key: 'tshock.tp.others', label: '传送其他玩家' },
      { key: 'tshock.tp.allothers', label: '传送所有人到自己' },
      { key: 'tshock.tp.pos', label: '传送到坐标' },
      { key: 'tshock.tp.getpos', label: '获取玩家位置' },
      { key: 'tshock.tp.npc', label: '传送到NPC' },
      { key: 'tshock.tp.home', label: '使用 /home' },
      { key: 'tshock.tp.spawn', label: '使用 /spawn' },
      { key: 'tshock.tp.block', label: '阻止被传送' },
      { key: 'tshock.tp.override', label: '无视传送阻止' },
      { key: 'tshock.tp.silent', label: '静默传送' },
      { key: 'tshock.tp.rod', label: '使用混沌传送杖' },
      { key: 'tshock.tp.wormhole', label: '使用虫洞药水' },
      { key: 'tshock.tp.pylon', label: '使用晶塔传送' },
      { key: 'tshock.tp.tppotion', label: '使用传送药水' },
      { key: 'tshock.tp.magicconch', label: '使用魔法海螺' },
      { key: 'tshock.tp.demonconch', label: '使用恶魔海螺' },
    ]
  },
  {
    key: 'tshock.world',
    label: '世界管理 (world)',
    children: [
      { key: 'tshock.world.modify', label: '修改世界' },
      { key: 'tshock.world.paint', label: '喷漆方块' },
      { key: 'tshock.world.save', label: '保存世界' },
      { key: 'tshock.world.info', label: '查看世界信息' },
      { key: 'tshock.world.setspawn', label: '设置出生点' },
      { key: 'tshock.world.setdungeon', label: '设置地牢位置' },
      { key: 'tshock.world.editspawn', label: '编辑出生区域' },
      { key: 'tshock.world.editregion', label: '编辑区域' },
      { key: 'tshock.world.grow', label: '种植植物' },
      { key: 'tshock.world.growevil', label: '种植腐化植物' },
      { key: 'tshock.world.hardmode', label: '切换困难模式' },
      { key: 'tshock.world.switchevil', label: '切换世界邪恶类型' },
      { key: 'tshock.world.toggleexpert', label: '切换专家模式' },
      { key: 'tshock.world.movenpc', label: '移动NPC' },
      { key: 'tshock.world.settleliquids', label: '稳定液体' },
      { key: 'tshock.world.rain', label: '切换降雨' },
      { key: 'tshock.world.sandstorm', label: '切换沙尘暴' },
      { key: 'tshock.world.wind', label: '修改风力' },
      { key: 'tshock.world.toggleparty', label: '切换派对事件' },
      { key: 'tshock.world.sethalloween', label: '强制万圣节模式' },
      { key: 'tshock.world.setxmas', label: '强制圣诞节模式' },
      { key: 'tshock.world.worldupgrades', label: '世界永久增益' },
      {
        key: 'tshock.world.events',
        label: '世界事件 (events)',
        children: [
          { key: 'tshock.world.events.bloodmoon', label: '血月事件' },
          { key: 'tshock.world.events.fullmoon', label: '满月事件' },
          { key: 'tshock.world.events.invasion', label: '入侵事件' },
          { key: 'tshock.world.events.meteor', label: '陨石事件' },
          { key: 'tshock.world.events.eclipse', label: '日食事件' },
          { key: 'tshock.world.events.sandstorm', label: '沙尘暴事件' },
          { key: 'tshock.world.events.rain', label: '降雨事件' },
          { key: 'tshock.world.events.lanternsnight', label: '灯笼夜事件' },
          { key: 'tshock.world.events.meteorshower', label: '流星雨事件' },
        ]
      },
      {
        key: 'tshock.world.time',
        label: '时间控制 (time)',
        children: [
          { key: 'tshock.world.time.set', label: '设置时间' },
          { key: 'tshock.world.time.bloodmoon', label: '强制血月' },
          { key: 'tshock.world.time.eclipse', label: '强制日食' },
          { key: 'tshock.world.time.fullmoon', label: '强制满月' },
          { key: 'tshock.world.time.dropmeteor', label: '掉落陨石' },
          { key: 'tshock.world.time.usesundial', label: '使用附魔日晷' },
          { key: 'tshock.world.time.usemoondial', label: '使用附魔月晷' },
        ]
      },
    ]
  },
  {
    key: 'tshock.journey',
    label: '旅途模式 (journey)',
    children: [
      { key: 'tshock.journey.godmode', label: '上帝模式' },
      { key: 'tshock.journey.placementrange', label: '放置范围' },
      { key: 'tshock.journey.setdifficulty', label: '设置世界难度' },
      { key: 'tshock.journey.biomespreadfreeze', label: '冻结腐化蔓延' },
      { key: 'tshock.journey.setspawnrate', label: '设置NPC生成率' },
      { key: 'tshock.journey.research', label: '贡献研究' },
      {
        key: 'tshock.journey.time',
        label: '旅途时间',
        children: [
          { key: 'tshock.journey.time.freeze', label: '冻结时间' },
          { key: 'tshock.journey.time.set', label: '设置时间' },
          { key: 'tshock.journey.time.setspeed', label: '设置时间速度' },
        ]
      },
      {
        key: 'tshock.journey.wind',
        label: '旅途风力',
        children: [
          { key: 'tshock.journey.wind.strength', label: '设置风力' },
          { key: 'tshock.journey.wind.freeze', label: '冻结风力' },
        ]
      },
      {
        key: 'tshock.journey.rain',
        label: '旅途降雨',
        children: [
          { key: 'tshock.journey.rain.strength', label: '设置降雨强度' },
          { key: 'tshock.journey.rain.freeze', label: '冻结降雨' },
        ]
      },
    ]
  },
  {
    key: 'tshock.ssc',
    label: 'SSC 角色 (ssc)',
    children: [
      { key: 'tshock.ssc.upload', label: '上传个人角色数据' },
      { key: 'tshock.ssc.upload.others', label: '上传他人角色数据' },
    ]
  },
  {
    key: 'tshock.misc',
    label: '杂项权限',
    children: [
      { key: 'tshock.canchat', label: '允许聊天' },
      { key: 'tshock.partychat', label: '队伍聊天' },
      { key: 'tshock.thirdperson', label: '第三人称说话' },
      { key: 'tshock.whisper', label: '私聊' },
      { key: 'tshock.sendemoji', label: '发送表情' },
      { key: 'tshock.info', label: '查看服务器信息' },
      { key: 'tshock.warp', label: '使用传送点' },
      { key: 'tshock.clear', label: '清除物品/弹幕' },
      { key: 'tshock.kill', label: '击杀玩家' },
      { key: 'tshock.slap', label: '拍打玩家' },
      { key: 'tshock.annoy', label: '骚扰玩家' },
      { key: 'tshock.heal', label: '治疗玩家' },
      { key: 'tshock.respawn', label: '自己重生' },
      { key: 'tshock.respawn.other', label: '让他人重生' },
      { key: 'tshock.godmode', label: '上帝模式' },
      { key: 'tshock.godmode.other', label: '给他人上帝模式' },
      { key: 'tshock.reservedslot', label: '保留位绕过' },
      { key: 'tshock.projectiles.usebanned', label: '使用封禁弹幕' },
      { key: 'tshock.tiles.usebanned', label: '使用封禁方块' },
      { key: 'tshock.accountinfo.check', label: '检查账号注册' },
      { key: 'tshock.accountinfo.details', label: '高级账号信息' },
      { key: 'tshock.synclocalarea', label: '与服务器重新同步' },
      { key: 'tshock.su', label: '临时超管提权' },
      { key: 'tshock.superadmin.user', label: '超管专属' },
    ]
  },
]

/** 递归收集所有叶节点权限 key */
export function collectAllPermissionKeys(nodes: PermissionNode[]): string[] {
  const keys: string[] = []
  for (const node of nodes) {
    if (node.children && node.children.length > 0) {
      keys.push(...collectAllPermissionKeys(node.children))
    } else {
      keys.push(node.key)
    }
  }
  return keys
}

/** 所有叶节点权限列表（扁平化） */
export const ALL_PERMISSION_KEYS = collectAllPermissionKeys(TSHOCK_PERMISSION_TREE)
