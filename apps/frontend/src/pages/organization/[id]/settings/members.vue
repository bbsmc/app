<template>
  <div class="normal-page__content">
    <div class="universal-card">
      <div class="label">
        <h3>
          <span class="label__title size-card-header">管理成员</span>
        </h3>
      </div>
      <span class="label">
        <span class="label__title">邀请成员</span>
        <span class="label__description">
          输入您想要邀请成为该团队成员的人员的 BBSMC 用户名。
        </span>
      </span>
      <div class="input-group">
        <input
          id="username"
          v-model="currentUsername"
          type="text"
          placeholder="用户名"
          :disabled="
            !isPermission(
              currentMember.organization_permissions,
              organizationPermissions.MANAGE_INVITES,
            )
          "
          @keypress.enter="() => onInviteTeamMember(organization.team, currentUsername)"
        />
        <label for="username" class="hidden">用户名</label>
        <Button
          color="primary"
          :disabled="
            !isPermission(
              currentMember.organization_permissions,
              organizationPermissions.MANAGE_INVITES,
            )
          "
          @click="() => onInviteTeamMember(organization.team_id, currentUsername)"
        >
          <UserPlusIcon />
          邀请
        </Button>
      </div>
      <div class="adjacent-input">
        <span class="label">
          <span class="label__title">离开团队</span>
          <span class="label__description"> 从该团队退出 </span>
        </span>
        <Button
          color="danger"
          :disabled="currentMember.is_owner"
          @click="() => onLeaveProject(organization.team_id, auth.user.id)"
        >
          <UserRemoveIcon />
          离开团队
        </Button>
      </div>
    </div>
    <div
      v-for="(member, index) in allTeamMembers"
      :key="member.user.id"
      class="member universal-card"
      :class="{ open: openTeamMembers.includes(member.user.id) }"
    >
      <div class="member-header">
        <div class="info">
          <Avatar :src="member.user.avatar_url" :alt="member.user.username" size="sm" circle />
          <div class="text">
            <nuxt-link :to="'/user/' + member.user.username" class="name">
              <p>{{ member.user.username }}</p>
              <CrownIcon v-if="member.is_owner" v-tooltip="'团队所有者'" />
            </nuxt-link>
            <p>{{ member.role }}</p>
          </div>
        </div>
        <div class="side-buttons">
          <Badge v-if="member.accepted" type="accepted" />
          <Badge v-else type="pending" />
          <Button
            icon-only
            transparent
            class="dropdown-icon"
            @click="
              openTeamMembers.indexOf(member.user.id) === -1
                ? openTeamMembers.push(member.user.id)
                : (openTeamMembers = openTeamMembers.filter((it) => it !== member.user.id))
            "
          >
            <DropdownIcon />
          </Button>
        </div>
      </div>
      <div class="content">
        <div class="adjacent-input">
          <label :for="`member-${member.user.id}-role`">
            <span class="label__title">角色</span>
            <span class="label__description"> 该成员在该团队中的头衔。 </span>
          </label>
          <input
            :id="`member-${member.user.id}-role`"
            v-model="member.role"
            type="text"
            :disabled="
              !isPermission(
                currentMember.organization_permissions,
                organizationPermissions.EDIT_MEMBER,
              )
            "
          />
        </div>
        <!--        <div class="adjacent-input">-->
        <!--          <label :for="`member-${member.user.id}-monetization-weight`">-->
        <!--            <span class="label__title">Monetization weight</span>-->
        <!--            <span class="label__description">-->
        <!--              Relative to all other members' monetization weights, this determines what portion of-->
        <!--              the organization projects' revenue goes to this member.-->
        <!--            </span>-->
        <!--          </label>-->
        <!--          <input-->
        <!--            :id="`member-${member.user.id}-monetization-weight`"-->
        <!--            v-model="member.payouts_split"-->
        <!--            type="number"-->
        <!--            :disabled="-->
        <!--              !isPermission(-->
        <!--                currentMember.organization_permissions,-->
        <!--                organizationPermissions.EDIT_MEMBER,-->
        <!--              )-->
        <!--            "-->
        <!--          />-->
        <!--        </div>-->
        <template v-if="!member.is_owner">
          <span class="label">
            <span class="label__title">资源权限</span>
          </span>
          <div class="permissions">
            <Checkbox
              v-for="[label, permission] in Object.entries(projectPermissions)"
              :key="permission"
              :model-value="isPermission(member.permissions, permission)"
              :disabled="
                !isPermission(
                  currentMember.organization_permissions,
                  organizationPermissions.EDIT_MEMBER_DEFAULT_PERMISSIONS,
                ) || !isPermission(currentMember.permissions, permission)
              "
              :label="permToLabel(label)"
              @update:model-value="allTeamMembers[index].permissions ^= permission"
            />
          </div>
        </template>
        <template v-if="!member.is_owner">
          <span class="label">
            <span class="label__title">团队权限</span>
          </span>
          <div class="permissions">
            <Checkbox
              v-for="[label, permission] in Object.entries(organizationPermissions)"
              :key="permission"
              :model-value="isPermission(member.organization_permissions, permission)"
              :disabled="
                !isPermission(
                  currentMember.organization_permissions,
                  organizationPermissions.EDIT_MEMBER,
                ) || !isPermission(currentMember.organization_permissions, permission)
              "
              :label="permToLabel(label)"
              @update:model-value="allTeamMembers[index].organization_permissions ^= permission"
            />
          </div>
        </template>
        <div class="input-group">
          <Button
            color="primary"
            :disabled="
              !isPermission(
                currentMember.organization_permissions,
                organizationPermissions.EDIT_MEMBER,
              )
            "
            @click="onUpdateTeamMember(organization.team_id, member)"
          >
            <SaveIcon />
            保存修改
          </Button>
          <Button
            v-if="!member.is_owner"
            color="danger"
            :disabled="
              !isPermission(
                currentMember.organization_permissions,
                organizationPermissions.EDIT_MEMBER,
              ) &&
              !isPermission(
                currentMember.organization_permissions,
                organizationPermissions.REMOVE_MEMBER,
              )
            "
            @click="onRemoveMember(organization.team_id, member)"
          >
            <UserRemoveIcon />
            移除成员
          </Button>
          <Button
            v-if="!member.is_owner && currentMember.is_owner && member.accepted"
            @click="() => onTransferOwnership(organization.team_id, member.user.id)"
          >
            <TransferIcon />
            转移所有权
          </Button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import {
  SaveIcon,
  TransferIcon,
  UserPlusIcon,
  UserXIcon as UserRemoveIcon,
  DropdownIcon,
} from "@modrinth/assets";
import { Button, Badge, Avatar, Checkbox } from "@modrinth/ui";
import { ref } from "vue";
import CrownIcon from "~/assets/images/utils/crown.svg?component";

import { removeTeamMember } from "~/helpers/teams.js";
import { isPermission } from "~/utils/permissions.ts";

const { organization, refresh: refreshOrganization, currentMember } = inject("organizationContext");

const auth = await useAuth();

const currentUsername = ref("");
const openTeamMembers = ref([]);

const allTeamMembers = ref(organization.value.members);

watch(
  () => organization.value,
  () => {
    allTeamMembers.value = organization.value.members;
  },
);

const projectPermissions = {
  UPLOAD_VERSION: 1 << 0,
  DELETE_VERSION: 1 << 1,
  EDIT_DETAILS: 1 << 2,
  EDIT_BODY: 1 << 3,
  MANAGE_INVITES: 1 << 4,
  REMOVE_MEMBER: 1 << 5,
  EDIT_MEMBER: 1 << 6,
  DELETE_PROJECT: 1 << 7,
  VIEW_ANALYTICS: 1 << 8,
  VIEW_PAYOUTS: 1 << 9,
  WIKI_EDIT: 1 << 10,
};

const organizationPermissions = {
  EDIT_DETAILS: 1 << 0,
  MANAGE_INVITES: 1 << 1,
  REMOVE_MEMBER: 1 << 2,
  EDIT_MEMBER: 1 << 3,
  ADD_PROJECT: 1 << 4,
  REMOVE_PROJECT: 1 << 5,
  DELETE_ORGANIZATION: 1 << 6,
  EDIT_MEMBER_DEFAULT_PERMISSIONS: 1 << 7,
};

const permToLabel = (key) => {
  if (key === "UPLOAD_VERSION") {
    return "上传版本";
  }
  if (key === "DELETE_VERSION") {
    return "删除版本";
  }
  if (key === "EDIT_DETAILS") {
    return "编辑详情";
  }
  if (key === "EDIT_BODY") {
    return "编辑正文";
  }
  if (key === "MANAGE_INVITES") {
    return "管理邀请";
  }
  if (key === "REMOVE_MEMBER") {
    return "移除成员";
  }
  if (key === "EDIT_MEMBER") {
    return "编辑成员";
  }
  if (key === "ADD_PROJECT") {
    return "添加项目";
  }
  if (key === "REMOVE_PROJECT") {
    return "移除项目";
  }
  if (key === "DELETE_ORGANIZATION") {
    return "删除组织";
  }
  if (key === "EDIT_DETAILS") {
    return "编辑详情";
  }
  if (key === "EDIT_MEMBER_DEFAULT_PERMISSIONS") {
    return "编辑默认权限";
  }

  if (key === "DELETE_PROJECT") {
    return "删除项目";
  }
  if (key === "VIEW_ANALYTICS") {
    return "查看分析";
  }
  if (key === "VIEW_PAYOUTS") {
    return "查看支付";
  }
  if (key === "WIKI_EDIT") {
    return "管理百科";
  }
  if (key === "EDIT_MEMBER_DEFAULT_PERMISSIONS") {
    return "编辑默认权限";
  }
  // return key;

  const o = key.split("_").join(" ");
  return o.charAt(0).toUpperCase() + o.slice(1).toLowerCase();
};

const leaveProject = async (teamId, uid) => {
  await removeTeamMember(teamId, uid);
  await navigateTo(`/organization/${organization.value.id}`);
};

const onLeaveProject = useClientTry(leaveProject);

const onInviteTeamMember = useClientTry(async (teamId, username) => {
  const user = await useBaseFetch(`user/${username}`);
  const data = {
    user_id: user.id.trim(),
  };
  await useBaseFetch(`team/${teamId}/members`, {
    method: "POST",
    body: data,
  });
  await refreshOrganization();
  currentUsername.value = "";
  addNotification({
    group: "main",
    title: "邀请成员",
    text: `${user.username} 已被邀请加入该团队。`,
    type: "success",
  });
});

const onRemoveMember = useClientTry(async (teamId, member) => {
  await removeTeamMember(teamId, member.user.id);
  await refreshOrganization();
  addNotification({
    group: "main",
    title: "成员移除",
    text: `${member.user.username} 已被从团队中移除`,
    type: "success",
  });
});

const onUpdateTeamMember = useClientTry(async (teamId, member) => {
  const data = !member.is_owner
    ? {
        permissions: member.permissions,
        organization_permissions: member.organization_permissions,
        role: member.role,
        payouts_split: member.payouts_split,
      }
    : {
        payouts_split: member.payouts_split,
        role: member.role,
      };
  await useBaseFetch(`team/${teamId}/members/${member.user.id}`, {
    method: "PATCH",
    body: data,
  });
  await refreshOrganization();
  addNotification({
    group: "main",
    title: "成员更新",
    text: `${member.user.username} 已被更新完成.`,
    type: "success",
  });
});

const onTransferOwnership = useClientTry(async (teamId, uid) => {
  const data = {
    user_id: uid,
  };
  await useBaseFetch(`team/${teamId}/owner`, {
    method: "PATCH",
    body: data,
  });
  await refreshOrganization();
  addNotification({
    group: "main",
    title: "所有权已转让",
    text: `${organization.value.name} 已成功转让.`,
    type: "success",
  });
});
</script>

<style lang="scss" scoped>
.member {
  .member-header {
    display: flex;
    justify-content: space-between;
    .info {
      display: flex;
      .text {
        margin: auto 0 auto 0.5rem;
        font-size: var(--font-size-sm);

        .name {
          font-weight: bold;

          display: flex;
          align-items: center;
          gap: 0.25rem;

          svg {
            color: var(--color-orange);
          }
        }

        p {
          margin: 0.2rem 0;
        }
      }
    }
    .side-buttons {
      display: flex;
      align-items: center;
      .dropdown-icon {
        margin-left: 1rem;
        svg {
          transition: 150ms ease transform;
        }
      }
    }
  }
  .content {
    display: none;
    flex-direction: column;
    padding-top: var(--gap-md);
    .main-info {
      margin-bottom: var(--gap-lg);
    }
    .permissions {
      margin-bottom: var(--gap-md);
      max-width: 45rem;
      display: grid;
      grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
      grid-gap: 0.5rem;
    }
  }
  &.open {
    .member-header {
      .dropdown-icon svg {
        transform: rotate(180deg);
      }
    }
    .content {
      display: flex;
    }
  }
}
:deep(.checkbox-outer) {
  button.checkbox {
    border: none;
  }
}
</style>
