<template>
  <div v-if="!route.name">
    <!-- 加载中或错误状态，不渲染任何内容 -->
  </div>
  <div v-else-if="route.name?.startsWith('type-id-settings')" class="normal-page">
    <div class="normal-page__sidebar">
      <aside class="universal-card">
        <Breadcrumbs
          current-title="设置"
          :link-stack="[
            {
              href: organization
                ? `/organization/${organization.slug}/settings/projects`
                : `/dashboard/projects`,
              label: '资源',
            },
            {
              href: `/${project.project_type}/${project.slug ? project.slug : project.id}`,
              label: project.title,
              allowTrimming: true,
            },
          ]"
        />
        <div class="settings-header">
          <Avatar
            :src="project.icon_url"
            :alt="project.title"
            size="sm"
            class="settings-header__icon"
          />
          <div class="settings-header__text">
            <h1 class="wrap-as-needed">
              {{ project.title }}
            </h1>
            <Badge :type="project.status" />
          </div>
        </div>
        <h2>资源设置</h2>
        <NavStack>
          <NavStackItem
            :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/settings`"
            label="基本"
          >
            <SettingsIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/tags`"
            label="标签"
          >
            <TagsIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/description`"
            label="介绍"
          >
            <DescriptionIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/license`"
            label="许可证"
          >
            <CopyrightIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/links`"
            label="链接"
          >
            <LinksIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/members`"
            label="成员"
          >
            <UsersIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            v-if="project.is_paid"
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/pricing`"
            label="定价"
          >
            <CurrencyIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            v-if="project.is_paid"
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/purchasers`"
            label="购买用户"
          >
            <UsersIcon aria-hidden="true" />
          </NavStackItem>
          <h3>视图</h3>
          <NavStackItem
            :link="`/${project.project_type}/${
              project.slug ? project.slug : project.id
            }/settings/analytics`"
            label="分析"
            chevron
          >
            <ChartIcon aria-hidden="true" />
          </NavStackItem>
          <h3>上传</h3>
          <NavStackItem
            :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/gallery`"
            label="渲染图"
            chevron
          >
            <GalleryIcon aria-hidden="true" />
          </NavStackItem>
          <NavStackItem
            :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/versions`"
            label="版本"
            chevron
          >
            <VersionIcon aria-hidden="true" />
          </NavStackItem>
          <h3>审核</h3>
          <NavStackItem
            :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/settings/translations`"
            label="翻译审核"
            chevron
          >
            <TranslateIcon aria-hidden="true" />
          </NavStackItem>
        </NavStack>
      </aside>
    </div>
    <div class="normal-page__content">
      <ProjectMemberHeader
        v-if="currentMember"
        :project="project"
        :versions="versions"
        :current-member="currentMember"
        :is-settings="route.name?.startsWith('type-id-settings')"
        :route-name="route.name"
        :set-processing="setProcessing"
        :collapsed="collapsedChecklist"
        :toggle-collapsed="() => (collapsedChecklist = !collapsedChecklist)"
        :all-members="allMembers"
        :update-members="updateMembers"
        :auth="auth"
        :tags="tags"
      />
      <NuxtPage
        v-model:project="project"
        v-model:versions="versions"
        v-model:wikis="wikis"
        v-model:featured-versions="featuredVersions"
        v-model:members="members"
        v-model:all-members="allMembers"
        v-model:dependencies="dependencies"
        v-model:organization="organization"
        :current-member="currentMember"
        :patch-project="patchProject"
        :patch-icon="patchIcon"
        :reset-project="resetProject"
        :reset-organization="resetOrganization"
        :reset-members="resetMembers"
        :route="route"
      />
    </div>
  </div>
  <div v-else class="experimental-styles-within">
    <NewModal ref="settingsModal">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
        <span class="text-lg font-extrabold text-contrast"> 设置 </span>
      </template>
    </NewModal>
    <NewModal ref="modalLicense" :header="project.license.name ? project.license.name : 'License'">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" no-shadow />
        <span class="text-lg font-extrabold text-contrast">
          {{ project.license.name ? project.license.name : "许可证" }}
        </span>
      </template>
      <div
        class="markdown-body"
        v-html="renderString(licenseText).isEmpty ? '正在加载许可证...' : renderString(licenseText)"
      />
    </NewModal>
    <div
      class="over-the-top-download-animation"
      :class="{ 'animation-hidden': !overTheTopDownloadAnimation }"
    >
      <div>
        <div
          class="animation-ring-3 flex items-center justify-center rounded-full border-4 border-solid border-brand bg-brand-highlight opacity-40"
        ></div>
        <div
          class="animation-ring-2 flex items-center justify-center rounded-full border-4 border-solid border-brand bg-brand-highlight opacity-60"
        ></div>
        <div
          class="animation-ring-1 flex items-center justify-center rounded-full border-4 border-solid border-brand bg-brand-highlight"
        >
          <DownloadIcon class="h-20 w-20 text-contrast" />
        </div>
      </div>
    </div>

    <NewModal ref="preReviewWiki">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
        <div class="truncate text-lg font-extrabold text-contrast">预览提交</div>
      </template>
      <ScrollablePanel
        :class="
          preIndexSetReview.length +
            preBodyReview.length +
            preADDReview.length +
            preSortReview.length +
            preREMOVEReview.length >
          4
            ? 'h-[30rem]'
            : ''
        "
      >
        <div class="flex flex-col gap-3" style="width: 500px">
          <div v-if="preIndexSetReview.length > 0" class="flex flex-col gap-2">
            <div class="flex flex-col gap-2">
              <label for="name">
                <span class="text-lg font-semibold text-contrast"> 设置主页: </span>
              </label>
              <span v-for="wiki in preIndexSetReview" :key="wiki.id" style="margin-left: 15px">{{
                wiki.title
              }}</span>
            </div>
          </div>

          <div v-if="preBodyReview.length > 0" class="flex flex-col gap-2">
            <div class="flex flex-col gap-2">
              <label for="name">
                <span class="text-lg font-semibold text-contrast"> 修改正文: </span>
              </label>
              <span v-for="wiki in preBodyReview" :key="wiki.id" style="margin-left: 15px">{{
                wiki.title
              }}</span>
            </div>
          </div>

          <div v-if="preSortReview.length > 0" class="flex flex-col gap-2">
            <div class="flex flex-col gap-2">
              <label for="name">
                <span class="text-lg font-semibold text-contrast"> 修改权重: </span>
              </label>
              <span v-for="wiki in preSortReview" :key="wiki.id" style="margin-left: 15px">{{
                wiki.title
              }}</span>
            </div>
          </div>

          <div v-if="preADDReview.length > 0" class="flex flex-col gap-2">
            <div class="flex flex-col gap-2">
              <label for="name">
                <span class="text-lg font-semibold text-contrast"> 新增页面: </span>
              </label>
              <span v-for="wiki in preADDReview" :key="wiki.id" style="margin-left: 15px">{{
                wiki.title
              }}</span>
            </div>
          </div>
          <div v-if="preREMOVEReview.length > 0" class="flex flex-col gap-2">
            <div class="flex flex-col gap-2">
              <label for="name">
                <span class="text-lg font-semibold text-contrast"> 移除页面: </span>
              </label>
              <span v-for="wiki in preREMOVEReview" :key="wiki.id" style="margin-left: 15px">{{
                wiki.title
              }}</span>
            </div>
          </div>
        </div>

        <span style="margin-top: 20px"></span>
      </ScrollablePanel>

      <!--      理由-->

      <div class="flex flex-col gap-2">
        <label for="name">
          <span class="text-lg font-semibold text-contrast"> 备注: </span>
        </label>
        <textarea v-model="submitWikiCacheMsg" type="text" placeholder="请输入提交的原因" />
      </div>
      <div v-if="wikis.cache.again_count > 0" class="mt-5 flex gap-2" style="font-size: 14px">
        已重复编辑了
        {{ wikis.cache.again_count }} 次，超过5次后将无法再次发起重复编辑或被拒绝后再次发起编辑
        <br />
        <br />
        第5次重复编辑后将被禁止发起新的编辑申请3小时
      </div>

      <div class="mt-5 flex gap-2">
        <ButtonStyled color="green">
          <button @click="submitConfirmForReview">
            <PlusIcon aria-hidden="true" />
            提交
          </button>
        </ButtonStyled>
        <ButtonStyled>
          <button style="margin-left: auto" @click="preReviewWiki.hide()">
            <XIcon aria-hidden="true" />
            取消
          </button>
        </ButtonStyled>
      </div>
    </NewModal>
    <NewModal ref="createWikiModal">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
        <div class="truncate text-lg font-extrabold text-contrast">创建WIKI页面</div>
      </template>

      <div class="flex flex-col gap-3" style="width: 500px">
        <div class="flex flex-col gap-2">
          <div class="flex flex-col gap-2">
            <label for="name">
              <span class="text-lg font-semibold text-contrast">
                页面标题
                <span class="text-brand-red">*</span>
              </span>
            </label>
            <input
              id="name"
              v-model="createWikiTitle"
              type="text"
              maxlength="64"
              placeholder="简短的标题..."
              autocomplete="off"
            />
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <label for="name">
            <span class="text-lg font-semibold text-contrast">
              页面SLUG
              <span class="text-brand-red">*</span>
            </span>
          </label>
          <input
            id="name"
            v-model="createWikiSlug"
            type="text"
            maxlength="64"
            placeholder="纯英文,用于在URL中使用不要使用除了横杠 - 之外的符号"
            autocomplete="off"
          />
        </div>

        <div class="flex flex-col gap-2">
          <label for="name">
            <span class="text-lg font-semibold text-contrast">
              上级目录(若新建的是子页面可选择上级)
              <!--            <span class="text-brand-red">*</span>-->
            </span>
          </label>
          <ButtonStyled v-if="wikis.cache.cache.length < 1">
            <div class="disabled button-like">
              <WrenchIcon aria-hidden="true" />
              未创建任何主目录
            </div>
          </ButtonStyled>
          <Accordion
            v-else
            ref="WikiFatherAccordion"
            class="accordion-with-bg"
            @on-open="
              () => {
                if (gameVersionAccordion) {
                  gameVersionAccordion.close();
                }
              }
            "
          >
            <template #title>
              <WikiIcon aria-hidden="true" />
              {{ createWikiFather ? `上级目录: ${createWikiFather.title}` : "选择上级目录" }}
            </template>
            <ScrollablePanel :class="project.loaders.length > 4 ? 'h-[15rem]' : ''">
              <ButtonStyled v-for="wiki in wikis.cache.cache" :key="wiki" color="brand">
                <button
                  @click="
                    () => {
                      createWikiFather = wiki;
                      WikiFatherAccordion.close();
                    }
                  "
                >
                  {{ wiki.title }}
                  <CheckIcon v-if="createWikiFather === wiki" />
                </button>
              </ButtonStyled>
            </ScrollablePanel>
          </Accordion>
        </div>

        <div class="mt-5 flex gap-2">
          <ButtonStyled color="brand">
            <button @click="createWiki">
              <PlusIcon aria-hidden="true" />
              创建页面
            </button>
          </ButtonStyled>
          <ButtonStyled>
            <button @click="createWikiModal.hide()">
              <XIcon aria-hidden="true" />
              取消
            </button>
          </ButtonStyled>
        </div>
      </div>
    </NewModal>

    <NewModal ref="downloadModal">
      <template #title>
        <Avatar :src="project.icon_url" :alt="project.title" class="icon" size="32px" />
        <div class="truncate text-lg font-extrabold text-contrast">下载 {{ project.title }}</div>
      </template>
      <template #default>
        <div class="mx-auto flex max-w-[40rem] flex-col gap-4 md:w-[30rem]">
          <div
            v-if="
              project.project_type !== 'plugin' ||
              project.loaders.some((x) => !tags.loaderData.allPluginLoaders.includes(x))
            "
            class="modrinth-app-section contents"
          ></div>

          <div class="mx-auto flex w-fit flex-col gap-2">
            <div v-if="project.game_versions.length > 0">
              <ButtonStyled v-if="project.game_versions.length === 1">
                <div class="disabled button-like">
                  <GameIcon aria-hidden="true" />
                  {{
                    currentGameVersion
                      ? `游戏版本: ${currentGameVersion}`
                      : "错误: 未找到任何游戏版本"
                  }}
                  <InfoIcon
                    v-tooltip="`${project.title} 仅可在 ${currentGameVersion} 运行`"
                    class="ml-auto size-5"
                  />
                </div>
              </ButtonStyled>
              <Accordion
                v-else
                ref="gameVersionAccordion"
                class="accordion-with-bg"
                @on-open="
                  () => {
                    if (platformAccordion) {
                      platformAccordion.close();
                    }
                  }
                "
              >
                <template #title>
                  <GameIcon aria-hidden="true" />
                  {{ currentGameVersion ? `游戏版本: ${currentGameVersion}` : "选择游戏版本" }}
                </template>
                <div class="iconified-input mb-2 flex w-full">
                  <label for="game-versions-filtering" hidden>搜索版本...</label>
                  <SearchIcon aria-hidden="true" />
                  <input
                    id="game-versions-filtering"
                    ref="gameVersionFilterInput"
                    v-model="versionFilter"
                    type="search"
                    autocomplete="off"
                    placeholder="搜索版本..."
                  />
                </div>
                <ScrollablePanel :class="project.game_versions.length > 4 ? 'h-[15rem]' : ''">
                  <ButtonStyled
                    v-for="version in project.game_versions
                      .filter(
                        (x) =>
                          (versionFilter && x.includes(versionFilter)) ||
                          (!versionFilter &&
                            (showAllVersions || (!x.includes('w') && !x.includes('-')))),
                      )
                      .slice()
                      .reverse()"
                    :key="version"
                    :color="currentGameVersion === version ? 'brand' : 'standard'"
                  >
                    <button
                      v-tooltip="
                        !possibleGameVersions.includes(version)
                          ? `${project.title} 该版本 ${version} 不支持在 ${formatCategory(currentPlatform)} 运行`
                          : null
                      "
                      :class="{
                        'looks-disabled !text-brand-red': !possibleGameVersions.includes(version),
                      }"
                      @click="
                        () => {
                          userSelectedGameVersion = version;
                          gameVersionAccordion.close();
                          if (!currentPlatform && platformAccordion) {
                            platformAccordion.open();
                          }
                        }
                      "
                    >
                      {{ version }}
                      <CheckIcon v-if="userSelectedGameVersion === version" />
                    </button>
                  </ButtonStyled>
                </ScrollablePanel>
                <Checkbox
                  v-model="showAllVersions"
                  class="mx-1"
                  :label="`显示全部版本`"
                  :disabled="!!versionFilter"
                />
              </Accordion>
            </div>

            <ButtonStyled
              v-if="project.loaders.length === 1 && project.project_type !== 'resourcepack'"
            >
              <div class="disabled button-like">
                <WrenchIcon aria-hidden="true" />
                {{
                  currentPlatform
                    ? `平台: ${formatCategory(currentPlatform)}`
                    : "错误: 未找到任何平台"
                }}
                <InfoIcon
                  v-tooltip="`${project.title} 仅可在 ${formatCategory(currentPlatform)} 运行`"
                  class="ml-auto size-5"
                />
              </div>
            </ButtonStyled>
            <Accordion
              v-else-if="project.project_type !== 'resourcepack'"
              ref="platformAccordion"
              class="accordion-with-bg"
              @on-open="
                () => {
                  if (gameVersionAccordion) {
                    gameVersionAccordion.close();
                  }
                }
              "
            >
              <template #title>
                <WrenchIcon aria-hidden="true" />
                {{ currentPlatform ? `平台: ${formatCategory(currentPlatform)}` : "选择运行平台" }}
              </template>
              <ScrollablePanel :class="project.loaders.length > 4 ? 'h-[15rem]' : ''">
                <ButtonStyled
                  v-for="platform in project.loaders.slice().reverse()"
                  :key="platform"
                  :color="currentPlatform === platform ? 'brand' : 'standard'"
                >
                  <button
                    v-tooltip="
                      !possiblePlatforms.includes(platform)
                        ? `${project.title} 不支持${currentGameVersion}的 ${formatCategory(platform)} `
                        : null
                    "
                    :class="{
                      'looks-disabled !text-brand-red': !possiblePlatforms.includes(platform),
                    }"
                    @click="
                      () => {
                        userSelectedPlatform = platform;

                        platformAccordion.close();
                        if (!currentGameVersion && gameVersionAccordion) {
                          gameVersionAccordion.open();
                        }
                      }
                    "
                  >
                    {{ formatCategory(platform) }}
                    <CheckIcon v-if="userSelectedPlatform === platform" />
                  </button>
                </ButtonStyled>
              </ScrollablePanel>
            </Accordion>
          </div>

          <AutomaticAccordion div class="flex flex-col gap-2">
            <VersionSummary
              v-if="filteredRelease"
              :version="filteredRelease"
              @on-download="onDownload"
              @on-navigate="downloadModal.hide"
            />
            <VersionSummary
              v-if="filteredBeta"
              :version="filteredBeta"
              @on-download="onDownload"
              @on-navigate="downloadModal.hide"
            />
            <VersionSummary
              v-if="filteredAlpha"
              :version="filteredAlpha"
              @on-download="onDownload"
              @on-navigate="downloadModal.hide"
            />
            <p
              v-if="
                currentPlatform &&
                currentGameVersion &&
                !filteredRelease &&
                !filteredBeta &&
                !filteredAlpha
              "
            >
              {{ currentGameVersion }} 和 {{ formatCategory(currentPlatform) }} 没有可用版本.
            </p>
          </AutomaticAccordion>

          <!-- 汉化包推荐 -->
          <TranslationPromo
            v-if="translationRecommendation"
            :translation-version="translationRecommendation"
            @navigate="navigateToTranslation"
          />

          <!-- 汉化包未及时更新提示：需要选择了版本且有可下载版本时才显示 -->
          <div
            v-else-if="
              project.translation_tracking &&
              !translationRecommendation &&
              currentGameVersion &&
              (filteredRelease || filteredBeta || filteredAlpha)
            "
            class="translation-pending-notice border-orange-500/50 bg-orange-500/10 rounded-2xl border border-solid p-4"
          >
            <div class="flex items-start gap-3">
              <InfoIcon class="text-orange-400 mt-0.5 size-5 shrink-0" />
              <div class="flex flex-col gap-1">
                <span class="font-bold text-contrast">当前版本暂无汉化包</span>
                <span class="text-sm text-secondary">
                  该版本的汉化包还未及时上传，可前往 QQ 群
                  <span class="text-orange-400 font-mono font-bold">1073724937</span>
                  反馈，我们将及时响应处理。
                </span>
              </div>
            </div>
          </div>

          <!-- 资源未被汉化提示：组织 6FNyvmc5 的资源且未开启汉化追踪 -->
          <div
            v-else-if="
              organization &&
              organization.id === '6FNyvmc5' &&
              !project.translation_tracking &&
              currentGameVersion &&
              (filteredRelease || filteredBeta || filteredAlpha)
            "
            class="translation-pending-notice rounded-2xl border border-solid border-gray-500/50 bg-gray-500/10 p-4"
          >
            <div class="flex items-start gap-3">
              <InfoIcon class="mt-0.5 size-5 shrink-0 text-gray-400" />
              <div class="flex flex-col gap-1">
                <span class="font-bold text-contrast">该资源暂未汉化</span>
                <span class="text-sm text-secondary">
                  如果需要汉化此资源，可前往 QQ 群
                  <span class="font-mono font-bold text-gray-400">1073724937</span>
                  反馈，我们将评估后进行汉化。
                </span>
              </div>
            </div>
          </div>

          <!-- 服务器推荐 -->
          <ServerPromo v-if="projectAffKey" @navigate="navigateToServer" />
        </div>
      </template>
    </NewModal>
    <CollectionCreateModal ref="modal_collection" :project-ids="[project.id]" />
    <div
      class="new-page sidebar revolution-layout"
      :class="{
        'alt-layout': route.fullPath.includes('/wikis') || route.fullPath.includes('/wiki/'),
      }"
    >
      <!-- ==================== IMMERSIVE HERO SECTION ==================== -->
      <div class="hero-section">
        <!-- Hero Background with Gallery Image -->
        <div class="hero-background">
          <img
            v-if="project.gallery && project.gallery.length > 0"
            :src="project.gallery[0].url"
            :alt="project.title"
            class="hero-bg-image"
          />
          <div class="hero-gradient-overlay"></div>
        </div>

        <!-- Hero Content -->
        <div class="hero-content">
          <div class="hero-main">
            <!-- Project Icon -->
            <div class="hero-icon-wrapper">
              <Avatar :src="project.icon_url" :alt="project.title" size="120px" class="hero-icon" />
            </div>

            <!-- Project Info -->
            <div class="hero-info">
              <div class="hero-title-row">
                <h1 class="hero-title">{{ project.title }}</h1>
                <Badge
                  v-if="auth.user && currentMember"
                  :type="project.status"
                  class="hero-status-badge"
                />
              </div>
              <p class="hero-description">{{ project.description }}</p>

              <!-- Tags & Stats Combined -->
              <div class="hero-meta">
                <!-- Quick Tags -->
                <div class="hero-tags">
                  <span
                    v-for="(category, index) in project.categories.slice(0, 3)"
                    :key="index"
                    class="hero-tag"
                  >
                    {{ formatCategory(category) }}
                  </span>
                  <span v-if="project.categories.length > 3" class="hero-tag hero-tag--more">
                    +{{ project.categories.length - 3 }}
                  </span>
                </div>

                <!-- Stats Inline -->
                <div class="hero-stats-inline">
                  <span class="stat-item"
                    ><DownloadIcon class="stat-icon" />{{ $formatNumber(project.downloads) }}</span
                  >
                  <span class="stat-item"
                    ><HeartIcon class="stat-icon" />{{ $formatNumber(project.followers) }}</span
                  >
                  <span class="stat-item"
                    ><CalendarIcon class="stat-icon" />{{
                      fromNow(project.approved || project.published)
                    }}</span
                  >
                </div>
              </div>
            </div>
          </div>

          <!-- Hero Actions -->
          <div class="hero-actions">
            <!-- 可以下载时显示下载按钮 -->
            <ButtonStyled v-if="canDownload" size="large" color="green" class="hero-download-btn">
              <button @click="(event) => onDownloadClick(event)">
                <DownloadIcon aria-hidden="true" />
                下载
              </button>
            </ButtonStyled>
            <!-- 需要购买时显示购买按钮 -->
            <ButtonStyled
              v-else-if="needsPurchase"
              size="large"
              color="brand"
              class="hero-download-btn"
            >
              <button @click="handlePurchaseClick">
                <CurrencyIcon aria-hidden="true" />
                购买
              </button>
            </ButtonStyled>
            <ButtonStyled v-if="projectAffKey" size="large" color="purple" type="transparent">
              <nuxt-link v-if="projectAffKey === 'pcl'" :to="`/pcl`" target="_blank">
                <ServerIcon aria-hidden="true" />
                联机
              </nuxt-link>
              <nuxt-link v-else :to="`/server?aff=${projectAffKey}`" target="_blank">
                <ServerIcon aria-hidden="true" />
                联机
              </nuxt-link>
            </ButtonStyled>
            <ButtonStyled size="large" circular :color="following ? 'red' : 'standard'">
              <button v-if="auth.user" @click="userFollowProject(project)">
                <HeartIcon :fill="following ? 'currentColor' : 'none'" aria-hidden="true" />
              </button>
              <nuxt-link v-else to="/auth/sign-in">
                <HeartIcon aria-hidden="true" />
              </nuxt-link>
            </ButtonStyled>
            <ButtonStyled size="large" circular>
              <PopoutMenu v-if="auth.user" from="top-right">
                <BookmarkIcon
                  :fill="
                    collections.some((x) => x.projects.includes(project.id))
                      ? 'currentColor'
                      : 'none'
                  "
                />
                <template #menu>
                  <input
                    v-model="displayCollectionsSearch"
                    type="text"
                    placeholder="搜索收藏..."
                    class="search-input menu-search"
                  />
                  <div v-if="collections.length > 0" class="collections-list">
                    <Checkbox
                      v-for="option in collections
                        .slice()
                        .sort((a, b) => a.name.localeCompare(b.name))"
                      :key="option.id"
                      :model-value="option.projects.includes(project.id)"
                      class="popout-checkbox"
                      @update:model-value="() => onUserCollectProject(option, project.id)"
                      >{{ option.name }}</Checkbox
                    >
                  </div>
                  <div v-else class="menu-text"><p class="popout-text">未找到任何收藏夹</p></div>
                  <button
                    class="btn collection-button"
                    @click="(event) => $refs.modal_collection.show(event)"
                  >
                    <PlusIcon aria-hidden="true" />创建收藏夹
                  </button>
                </template>
              </PopoutMenu>
              <nuxt-link v-else to="/auth/sign-in"><BookmarkIcon aria-hidden="true" /></nuxt-link>
            </ButtonStyled>
            <!-- Settings Button -->
            <ButtonStyled v-if="auth.user && currentMember" size="large" circular>
              <nuxt-link
                :to="`/${project.project_type}/${project.slug ? project.slug : project.id}/settings`"
              >
                <SettingsIcon aria-hidden="true" />
              </nuxt-link>
            </ButtonStyled>
            <!-- More Options Menu -->
            <ButtonStyled size="large" circular type="transparent">
              <OverflowMenu
                :options="[
                  {
                    id: 'analytics',
                    link: `/${project.project_type}/${project.slug ? project.slug : project.id}/settings/analytics`,
                    hoverOnly: true,
                    shown: auth.user && !!currentMember,
                  },
                  {
                    divider: true,
                    shown: auth.user && !!currentMember,
                  },
                  {
                    id: 'moderation-checklist',
                    action: () => (showModerationChecklist = true),
                    color: 'orange',
                    hoverOnly: true,
                    shown:
                      auth.user &&
                      tags.staffRoles.includes(auth.user.role) &&
                      !showModerationChecklist,
                  },
                  {
                    divider: true,
                    shown:
                      auth.user &&
                      tags.staffRoles.includes(auth.user.role) &&
                      !showModerationChecklist,
                  },
                  {
                    id: 'report',
                    action: () =>
                      auth.user ? reportProject(project.id) : navigateTo('/auth/sign-in'),
                    color: 'red',
                    hoverOnly: true,
                    shown: !currentMember,
                  },
                  { id: 'copy-id', action: () => copyId() },
                ]"
                aria-label="More options"
              >
                <MoreVerticalIcon aria-hidden="true" />
                <template #analytics>
                  <ChartIcon aria-hidden="true" />
                  分析
                </template>
                <template #moderation-checklist>
                  <ScaleIcon aria-hidden="true" />
                  审查项目
                </template>
                <template #report>
                  <ReportIcon aria-hidden="true" />
                  举报
                </template>
                <template #copy-id>
                  <ClipboardCopyIcon aria-hidden="true" />
                  复制资源 ID
                </template>
              </OverflowMenu>
            </ButtonStyled>
          </div>
        </div>
      </div>

      <!-- ==================== LEGACY HEADER (HIDDEN, KEPT FOR COMPATIBILITY) ==================== -->
      <div class="normal-page__header relative my-4" style="display: none">
        <ContentPageHeader>
          <template #icon>
            <Avatar :src="project.icon_url" :alt="project.title" size="96px" />
          </template>
          <template #title>
            {{ project.title }}
          </template>
          <template #title-suffix>
            <Badge v-if="auth.user && currentMember" :type="project.status" class="status-badge" />
          </template>
          <template #summary>
            {{ project.description }}
          </template>
          <template #stats>
            <div
              class="flex items-center gap-2 border-0 border-r border-solid border-button-bg pr-4 font-semibold"
            >
              <DownloadIcon class="h-6 w-6 text-secondary" />
              {{ $formatNumber(project.downloads) }}
            </div>
            <div
              class="flex items-center gap-2 border-0 border-solid border-button-bg pr-4 md:border-r"
            >
              <HeartIcon class="h-6 w-6 text-secondary" />
              <span class="font-semibold">
                {{ $formatNumber(project.followers) }}
              </span>
            </div>
            <div class="hidden items-center gap-2 md:flex">
              <TagsIcon class="h-6 w-6 text-secondary" />
              <div class="flex flex-wrap gap-2">
                <div
                  v-for="(category, index) in project.categories"
                  :key="index"
                  class="tag-list__item"
                >
                  {{ formatCategory(category) }}
                </div>
              </div>
            </div>
          </template>
          <template #actions>
            <div class="hidden sm:contents">
              <!-- 可以下载时显示下载按钮 -->
              <ButtonStyled v-if="canDownload" size="large" color="green">
                <button
                  @click="
                    (event) => {
                      onDownloadClick(event);
                    }
                  "
                >
                  <DownloadIcon aria-hidden="true" />
                  下载
                </button>
              </ButtonStyled>
              <!-- 需要购买时显示购买按钮 -->
              <ButtonStyled v-else-if="needsPurchase" size="large" color="brand">
                <button @click="scrollToPurchase">
                  <CurrencyIcon aria-hidden="true" />
                  购买
                </button>
              </ButtonStyled>

              <ButtonStyled v-if="projectAffKey" size="large" color="purple" type="transparent">
                <nuxt-link v-if="projectAffKey === 'pcl'" :to="`/pcl`" target="_blank">
                  <ServerIcon aria-hidden="true" />
                  联机搭建
                </nuxt-link>
                <nuxt-link v-else :to="`/server?aff=${projectAffKey}`" target="_blank">
                  <ServerIcon aria-hidden="true" />
                  联机搭建
                </nuxt-link>
              </ButtonStyled>
            </div>
            <div class="contents sm:hidden">
              <!-- 可以下载时显示下载按钮 -->
              <ButtonStyled
                v-if="canDownload"
                size="large"
                circular
                :color="route.name === 'type-id-version-version' ? `standard` : `brand`"
              >
                <button
                  aria-label="Download"
                  class="flex sm:hidden"
                  @click="(event) => onDownloadClick(event)"
                >
                  <DownloadIcon aria-hidden="true" />
                </button>
              </ButtonStyled>
              <!-- 需要购买时显示购买按钮 -->
              <ButtonStyled v-else-if="needsPurchase" size="large" circular color="brand">
                <button aria-label="Purchase" class="flex sm:hidden" @click="handlePurchaseClick">
                  <CurrencyIcon aria-hidden="true" />
                </button>
              </ButtonStyled>

              <ButtonStyled v-if="projectAffKey" size="large" color="purple" type="transparent">
                <nuxt-link v-if="projectAffKey === 'pcl'" :to="`/pcl`" target="_blank">
                  <ServerIcon aria-hidden="true" />
                  联机搭建
                </nuxt-link>
                <nuxt-link v-else :to="`/server?aff=${projectAffKey}`" target="_blank">
                  <ServerIcon aria-hidden="true" />
                  联机搭建
                </nuxt-link>
              </ButtonStyled>
            </div>
            <ButtonStyled
              size="large"
              circular
              :color="following ? 'red' : 'standard'"
              color-fill="none"
              hover-color-fill="background"
            >
              <button
                v-if="auth.user"
                v-tooltip="following ? `取消关注` : `关注`"
                :aria-label="following ? `取消关注` : `关注`"
                @click="userFollowProject(project)"
              >
                <HeartIcon :fill="following ? 'currentColor' : 'none'" aria-hidden="true" />
              </button>
              <nuxt-link v-else v-tooltip="'Follow'" to="/auth/sign-in" aria-label="Follow">
                <HeartIcon aria-hidden="true" />
              </nuxt-link>
            </ButtonStyled>
            <ButtonStyled size="large" circular>
              <PopoutMenu v-if="auth.user" v-tooltip="'保存'" from="top-right" aria-label="Save">
                <BookmarkIcon
                  aria-hidden="true"
                  :fill="
                    collections.some((x) => x.projects.includes(project.id))
                      ? 'currentColor'
                      : 'none'
                  "
                />
                <template #menu>
                  <input
                    v-model="displayCollectionsSearch"
                    type="text"
                    placeholder="搜索收藏..."
                    class="search-input menu-search"
                  />
                  <div v-if="collections.length > 0" class="collections-list">
                    <Checkbox
                      v-for="option in collections
                        .slice()
                        .sort((a, b) => a.name.localeCompare(b.name))"
                      :key="option.id"
                      :model-value="option.projects.includes(project.id)"
                      class="popout-checkbox"
                      @update:model-value="() => onUserCollectProject(option, project.id)"
                    >
                      {{ option.name }}
                    </Checkbox>
                  </div>
                  <div v-else class="menu-text">
                    <p class="popout-text">未找到任何收收藏夹</p>
                  </div>
                  <button
                    class="btn collection-button"
                    @click="(event) => $refs.modal_collection.show(event)"
                  >
                    <PlusIcon aria-hidden="true" />
                    创建收藏夹
                  </button>
                </template>
              </PopoutMenu>
              <nuxt-link v-else v-tooltip="'保存'" to="/auth/sign-in" aria-label="Save">
                <BookmarkIcon aria-hidden="true" />
              </nuxt-link>
            </ButtonStyled>
            <ButtonStyled v-if="auth.user && currentMember" size="large" circular>
              <nuxt-link
                :to="`/${project.project_type}/${project.slug ? project.slug : project.id}/settings`"
              >
                <SettingsIcon aria-hidden="true" />
              </nuxt-link>
            </ButtonStyled>
            <ButtonStyled size="large" circular type="transparent">
              <OverflowMenu
                :options="[
                  {
                    id: 'analytics',
                    link: `/${project.project_type}/${project.slug ? project.slug : project.id}/settings/analytics`,
                    hoverOnly: true,
                    shown: auth.user && !!currentMember,
                  },
                  {
                    divider: true,
                    shown: auth.user && !!currentMember,
                  },
                  {
                    id: 'moderation-checklist',
                    action: () => (showModerationChecklist = true),
                    color: 'orange',
                    hoverOnly: true,
                    shown:
                      auth.user &&
                      tags.staffRoles.includes(auth.user.role) &&
                      !showModerationChecklist,
                  },
                  {
                    divider: true,
                    shown:
                      auth.user &&
                      tags.staffRoles.includes(auth.user.role) &&
                      !showModerationChecklist,
                  },
                  {
                    id: 'report',
                    action: () =>
                      auth.user ? reportProject(project.id) : navigateTo('/auth/sign-in'),
                    color: 'red',
                    hoverOnly: true,
                    shown: !currentMember,
                  },
                  { id: 'copy-id', action: () => copyId() },
                ]"
                aria-label="More options"
              >
                <MoreVerticalIcon aria-hidden="true" />
                <template #analytics>
                  <ChartIcon aria-hidden="true" />
                  分析
                </template>
                <template #moderation-checklist>
                  <ScaleIcon aria-hidden="true" />
                  审查项目
                </template>
                <template #report>
                  <ReportIcon aria-hidden="true" />
                  举报
                </template>
                <template #copy-id>
                  <ClipboardCopyIcon aria-hidden="true" />
                  复制资源 ID
                </template>
              </OverflowMenu>
            </ButtonStyled>
          </template>
        </ContentPageHeader>
        <ProjectMemberHeader
          v-if="currentMember"
          :project="project"
          :versions="versions"
          :current-member="currentMember"
          :is-settings="route.name?.startsWith('type-id-settings')"
          :route-name="route.name"
          :set-processing="setProcessing"
          :collapsed="collapsedChecklist"
          :toggle-collapsed="() => (collapsedChecklist = !collapsedChecklist)"
          :all-members="allMembers"
          :update-members="updateMembers"
          :auth="auth"
          :tags="tags"
        />
        <MessageBanner v-if="project.status === 'archived'" message-type="warning" class="mb-4">
          {{ project.title }} 已停更. {{ project.title }} 将不会再进行任何更新,除非作者取消停更状态
        </MessageBanner>
      </div>
      <!--      百科导航栏    -->
      <div
        v-if="
          (route.fullPath.includes('/wikis') || route.fullPath.includes('/wiki/')) &&
          (wikis.wikis.length > 0 || wikis.is_editor)
        "
        class="normal-page__sidebar"
      >
        <aside class="universal-card">
          <div v-if="wikis.is_editor && wikis.is_editor_user && wikis.cache.status === 'draft'">
            <ButtonStyled type="standard" @click="(event) => createWikiModal.show(event)">
              <nuxt-link> 新建页面 </nuxt-link>
            </ButtonStyled>
            <ButtonStyled type="standard" @click="submitForReview">
              <nuxt-link style="margin-top: 10px"> 提交草稿审核 </nuxt-link>
            </ButtonStyled>
            <hr />
            <NavStack>
              <div v-for="wiki in wikis.cache.cache" :key="wiki.id" class="my-1">
                <NuxtLink
                  class="nav-link button-base"
                  :to="`/${project.project_type}/${project.slug ? project.slug : project.id}/wiki/${wiki.slug}`"
                >
                  <div class="nav-content">
                    <slot />
                    <h3>{{ wiki.title }}</h3>
                  </div>
                </NuxtLink>
                <NavStackItem
                  v-for="w in wiki.child"
                  :key="w.id"
                  :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/wiki/${w.slug}`"
                  :label="w.title"
                />
              </div>
            </NavStack>
          </div>

          <NavStack v-else>
            <div v-for="wiki in wikis.wikis" :key="wiki.id" class="my-1">
              <NuxtLink
                class="nav-link button-base"
                :to="`/${project.project_type}/${project.slug ? project.slug : project.id}/wiki/${wiki.slug}`"
              >
                <div class="nav-content">
                  <slot />
                  <h3>{{ wiki.title }}</h3>
                </div>
              </NuxtLink>
              <NavStackItem
                v-for="w in wiki.child"
                :key="w.id"
                :link="`/${project.project_type}/${project.slug ? project.slug : project.id}/wiki/${w.slug}`"
                :label="w.title"
              />
            </div>
          </NavStack>
          <!-- Wiki 页面付费提示 -->
          <PurchaseButton
            v-if="wikis.requires_purchase"
            :project="project"
            :current-member="currentMember"
            class="mt-4"
            @purchase-success="() => router.go(0)"
          />
        </aside>
      </div>
      <!--      侧边栏-->
      <div v-else class="normal-page__sidebar">
        <!-- 付费资源购买区域 -->
        <PurchaseButton
          v-if="project.is_paid"
          :project="project"
          :current-member="currentMember"
          class="purchase-card"
          @purchase-success="() => router.go(0)"
        />

        <div
          v-if="project.project_type !== 'language'"
          class="card flex-card experimental-styles-within"
        >
          <h2>{{ formatMessage(compatibilityMessages.title) }}</h2>
          <section>
            <h3>{{ formatMessage(compatibilityMessages.minecraftJava) }}</h3>
            <div class="tag-list">
              <div
                v-for="version in getVersionsToDisplay(project)"
                :key="`version-tag-${version}`"
                class="tag-list__item"
              >
                {{ version }}
              </div>
            </div>
          </section>
          <section
            v-if="project.project_type !== 'resourcepack' && project.project_type !== 'language'"
          >
            <h3>{{ formatMessage(compatibilityMessages.platforms) }}</h3>
            <div class="tag-list">
              <div
                v-for="platform in project.loaders"
                :key="`platform-tag-${platform}`"
                :class="`tag-list__item`"
                :style="`--_color: var(--color-platform-${platform})`"
              >
                <svg v-html="tags.loaders.find((x) => x.name === platform).icon"></svg>
                {{ formatCategory(platform) }}
              </div>
            </div>
          </section>
          <section
            v-if="
              (project.actualProjectType === 'mod' || project.project_type === 'modpack') &&
              !(project.client_side === 'unsupported' && project.server_side === 'unsupported') &&
              !(project.client_side === 'unknown' && project.server_side === 'unknown')
            "
          >
            <h3>{{ formatMessage(compatibilityMessages.environments) }}</h3>
            <div class="tag-list">
              <div
                v-if="
                  (project.client_side === 'required' && project.server_side !== 'required') ||
                  (project.client_side === 'optional' && project.server_side === 'optional')
                "
                class="tag-list__item"
              >
                <ClientIcon aria-hidden="true" />
                客户端
              </div>
              <div
                v-if="
                  (project.server_side === 'required' && project.client_side !== 'required') ||
                  (project.client_side === 'optional' && project.server_side === 'optional')
                "
                class="tag-list__item"
              >
                <ServerIcon aria-hidden="true" />
                服务端
              </div>
              <div v-if="false" class="tag-list__item">
                <UserIcon aria-hidden="true" />
                单人
              </div>
              <div
                v-if="
                  project.project_type !== 'datapack' &&
                  ((project.client_side === 'required' && project.server_side === 'required') ||
                    project.client_side === 'optional' ||
                    (project.client_side === 'required' && project.server_side === 'optional') ||
                    project.server_side === 'optional' ||
                    (project.server_side === 'required' && project.client_side === 'optional'))
                "
                class="tag-list__item"
              >
                <MonitorSmartphoneIcon aria-hidden="true" />
                客户端和服务端
              </div>
            </div>
          </section>
        </div>

        <div
          v-if="
            project.issues_url ||
            project.source_url ||
            project.wiki_url ||
            project.discord_url ||
            project.donation_urls.length > 0
          "
          class="card flex-card experimental-styles-within"
        >
          <h2>其他链接</h2>
          <div class="links-list">
            <a
              v-if="project.issues_url"
              :href="project.issues_url"
              :target="$external()"
              rel="noopener nofollow ugc"
            >
              <IssuesIcon aria-hidden="true" />
              反馈问题
              <ExternalIcon aria-hidden="true" class="external-icon" />
            </a>
            <a
              v-if="project.source_url"
              :href="project.source_url"
              :target="$external()"
              rel="noopener nofollow ugc"
            >
              <CodeIcon aria-hidden="true" />
              查看源码
              <ExternalIcon aria-hidden="true" class="external-icon" />
            </a>
            <a
              v-if="project.wiki_url"
              :href="project.wiki_url"
              :target="$external()"
              rel="noopener nofollow ugc"
            >
              <WikiIcon aria-hidden="true" />
              访问 wiki
              <ExternalIcon aria-hidden="true" class="external-icon" />
            </a>
            <hr
              v-if="
                (project.issues_url ||
                  project.source_url ||
                  project.wiki_url ||
                  project.discord_url) &&
                project.donation_urls.length > 0
              "
            />
            <a
              v-for="(donation, index) in project.donation_urls"
              :key="index"
              :href="donation.url"
              :target="$external()"
              rel="noopener nofollow ugc"
            >
              <AifadianIcon v-if="donation.id === 'afdian'" aria-hidden="true" />
              <BiliBiliIcon v-else-if="donation.id === 'bilibili'" aria-hidden="true" />
              <WebIcon2 v-else-if="donation.id === 'other'" aria-hidden="true" />
              <WebIcon2 v-else-if="donation.id === 'site'" aria-hidden="true" />
              <ModrinthIcon2 v-else-if="donation.id === 'modrinth'" aria-hidden="true" />
              <QQPDIcon v-else-if="donation.id === 'pd-qq'" aria-hidden="true" />
              <OopzIcon v-else-if="donation.id === 'oopz'" aria-hidden="true" />
              <KookIcon v-else-if="donation.id === 'kook'" aria-hidden="true" />
              <SpigotMcIcon v-else-if="donation.id === 'spigotmc'" aria-hidden="true" />
              <QuarkIcon v-else-if="donation.id === 'quark'" aria-hidden="true" />
              <BaiduIcon v-else-if="donation.id === 'baidu'" aria-hidden="true" />
              <CurseforgeIcon v-else-if="donation.id === 'curseforge'" aria-hidden="true" />
              <McmodIcon v-else-if="donation.id === 'mcmod'" aria-hidden="true" />
              <Mc9yIcon v-else-if="donation.id === 'mc9y'" aria-hidden="true" />
              <KoFiIcon v-else-if="donation.id === 'ko-fi'" aria-hidden="true" />
              <PayPalIcon v-else-if="donation.id === 'paypal'" aria-hidden="true" />
              <OpenCollectiveIcon
                v-else-if="donation.id === 'open-collective'"
                aria-hidden="true"
              />
              <HeartIcon v-else-if="donation.id === 'github'" />
              <CurrencyIcon v-else />

              <span>{{ webDisplayLabel(donation.id) }}</span>
              <ExternalIcon aria-hidden="true" class="external-icon" />
            </a>
          </div>
        </div>
        <div class="card flex-card experimental-styles-within">
          <h2>
            {{
              organization
                ? ["bbsmc", "bbsmc-2", "bbsmc-3"].includes(organization.slug)
                  ? "搬运团队"
                  : "创作团队"
                : "创作者"
            }}
          </h2>
          <div class="details-list">
            <template v-if="organization">
              <nuxt-link
                class="details-list__item details-list__item--type-large"
                :to="`/organization/${organization.slug}`"
              >
                <Avatar :src="organization.icon_url" :alt="organization.name" size="32px" />
                <div class="rows">
                  <span>
                    {{ organization.name }}
                  </span>
                  <span class="details-list__item__text--style-secondary">团队</span>
                </div>
              </nuxt-link>
              <hr v-if="members.length > 0" />
            </template>
            <nuxt-link
              v-for="member in members"
              :key="`member-${member.id}`"
              class="details-list__item details-list__item--type-large"
              :to="'/user/' + member.user.username"
            >
              <Avatar :src="member.avatar_url" :alt="member.name" size="32px" circle />
              <div class="rows">
                <span class="flex items-center gap-1">
                  {{ member.name }}
                  <CrownIcon
                    v-if="member.is_owner"
                    v-tooltip="formatMessage(creatorsMessages.owner)"
                    class="text-brand-orange"
                  />
                </span>
                <span class="details-list__item__text--style-secondary">{{ member.role }}</span>
              </div>
            </nuxt-link>
          </div>
        </div>
        <!-- 汉化追踪标记 -->
        <div v-if="project.translation_tracking" class="card flex-card experimental-styles-within">
          <div class="flex items-center gap-2">
            <TranslateIcon class="h-5 w-5 text-brand-green" aria-hidden="true" />
            <h2 class="!mb-0">汉化追踪中</h2>
          </div>
          <p style="font-size: 0.875rem; color: var(--color-text); line-height: 1.6; margin: 0">
            此项目正在进行汉化追踪，将会定期同步上游更新并更新汉化内容。
          </p>
          <!-- 汉化包项目卡片 -->
          <nuxt-link
            v-if="translationPackProject"
            :to="`/${translationPackProject.project_type}/${translationPackProject.slug || translationPackProject.id}`"
            class="translation-pack-card"
          >
            <Avatar :src="translationPackProject.icon_url" alt="translation-pack-icon" size="sm" />
            <div class="translation-pack-info">
              <span class="translation-pack-title">{{ translationPackProject.title }}</span>
              <span class="translation-pack-desc">{{ translationPackProject.description }}</span>
            </div>
          </nuxt-link>
        </div>

        <!-- 为目标追踪资源标记（当前项目是某个项目的汉化包） -->
        <div v-if="translationSourceProject" class="card flex-card experimental-styles-within">
          <div class="flex items-center gap-2">
            <TranslateIcon class="h-5 w-5 text-brand-green" aria-hidden="true" />
            <h2 class="!mb-0">追踪汉化包</h2>
          </div>
          <p style="font-size: 0.875rem; color: var(--color-text); line-height: 1.6; margin: 0">
            此资源是以下整合包的汉化包。
          </p>
          <!-- 原始项目卡片 -->
          <nuxt-link
            :to="`/${translationSourceProject.project_type}/${translationSourceProject.slug || translationSourceProject.id}`"
            class="translation-pack-card"
          >
            <Avatar :src="translationSourceProject.icon_url" alt="source-project-icon" size="sm" />
            <div class="translation-pack-info">
              <span class="translation-pack-title">{{ translationSourceProject.title }}</span>
              <span class="translation-pack-desc">{{ translationSourceProject.description }}</span>
            </div>
          </nuxt-link>
        </div>

        <!-- 资源未被汉化提示：组织 6FNyvmc5 的资源且未开启汉化追踪 -->
        <div
          v-if="
            organization &&
            organization.id === '6FNyvmc5' &&
            !project.translation_tracking &&
            !translationSourceProject
          "
          class="card flex-card experimental-styles-within"
        >
          <div class="flex items-center gap-2">
            <TranslateIcon class="h-5 w-5 text-gray-400" aria-hidden="true" />
            <h2 class="!mb-0">暂未汉化</h2>
          </div>
          <p style="font-size: 0.875rem; color: var(--color-text); line-height: 1.6; margin: 0">
            该资源暂未进行汉化，如果需要汉化此资源，可前往 QQ 群
            <span class="font-mono font-bold">1073724937</span>
            反馈，我们将评估后进行汉化。
          </p>
        </div>

        <div
          v-if="organization && ['bbsmc', 'bbsmc-2', 'bbsmc-3'].includes(organization.slug)"
          class="card flex-card experimental-styles-within"
        >
          <h2>搬运资源声明</h2>
          <p style="font-size: 0.875rem; color: var(--color-text); line-height: 1.6; margin: 0">
            对于可进行 JAR
            文件搬运的许可证，我们提供站内下载服务；其他资源会跳转到原帖下载。资源更新可能不及时，建议前往资源内提供的原帖链接下载最新版本。
          </p>
        </div>
        <div
          v-if="organization && organization.slug === 'bbsmc-cn'"
          class="card flex-card experimental-styles-within"
        >
          <h2>BBSMC汉化组</h2>
          <p style="font-size: 0.875rem; color: var(--color-text); line-height: 1.6; margin: 0">
            BBSMC汉化组是一个专注于Minecraft整合包汉化的团队，我们借助AI翻译技术 +
            1000万文本的MC专业词汇结合人工校对，力求在效率与质量之间找到最佳平衡。我们针对每个资源的版本进行针对性汉化，在下载的时候一定要针对特定的版本下载特定版本的汉化包，不要使用跨版本的汉化包。比如整合包的版本是
            1.0.1 就不要使用针对 1.0.0
            版本的汉化包，会导致很多问题无法正常游戏游玩。缺少版本需求请加入汉化组的QQ群
            <span class="font-mono font-bold">1073724937</span> 提需求，我们会快速响应。
          </p>
        </div>
        <div class="card flex-card experimental-styles-within">
          <h2>{{ formatMessage(detailsMessages.title) }}</h2>
          <div class="details-list">
            <div class="details-list__item">
              <BookTextIcon aria-hidden="true" />
              <div>
                许可证
                <a
                  v-if="project.license.url"
                  class="text-link hover:underline"
                  :href="project.license.url"
                  :target="$external()"
                  rel="noopener nofollow ugc"
                >
                  {{ licenseIdDisplay }}
                  <ExternalIcon aria-hidden="true" class="external-icon ml-1 mt-[-1px] inline" />
                </a>
                <span
                  v-else-if="
                    project.license.id === 'LicenseRef-All-Rights-Reserved' ||
                    !project.license.id.includes('LicenseRef')
                  "
                  class="text-link hover:underline"
                  @click="(event) => getLicenseData(event)"
                >
                  {{ licenseIdDisplay }}
                </span>
                <span v-else>{{ licenseIdDisplay }}</span>
              </div>
            </div>

            <!--            发布-->
            <div
              v-if="project.approved"
              v-tooltip="formatDateTime(project.approved)"
              class="details-list__item"
            >
              <CalendarIcon aria-hidden="true" />
              <div>发布于 {{ formatDate(project.approved) }}</div>
            </div>

            <!--            提交-->
            <div v-else v-tooltip="formatDateTime(project.published)" class="details-list__item">
              <CalendarIcon aria-hidden="true" />
              <div>提交于 {{ formatDate(project.published) }}</div>
            </div>

            <!--            发布-->
            <div
              v-if="project.status === 'processing' && project.queued"
              v-tooltip="formatDateTime(project.queued)"
              class="details-list__item"
            >
              <ScaleIcon aria-hidden="true" />
              <div>发布于 {{ formatDate(project.queued) }}</div>
            </div>

            <!--            更新-->
            <div
              v-if="versions.length > 0 && project.updated"
              v-tooltip="formatDateTime(project.updated)"
              class="details-list__item"
            >
              <VersionIcon aria-hidden="true" />
              <div>更新于 {{ formatDate(project.updated) }}</div>
            </div>
          </div>
        </div>
      </div>
      <div class="normal-page__content">
        <div class="overflow-x-auto">
          <NavTabs :links="navLinks" class="mb-4" />
        </div>
        <NuxtPage
          v-model:project="project"
          v-model:versions="versions"
          v-model:wikis="wikis"
          v-model:featured-versions="featuredVersions"
          v-model:members="members"
          v-model:all-members="allMembers"
          v-model:dependencies="dependencies"
          v-model:organization="organization"
          :current-member="currentMember"
          :reset-project="resetProject"
          :reset-organization="resetOrganization"
          :reset-members="resetMembers"
          :route="route"
          @on-download="triggerDownloadAnimation"
        />
      </div>
    </div>
    <ModerationChecklist
      v-if="auth.user && tags.staffRoles.includes(auth.user.role) && showModerationChecklist"
      :project="project"
      :future-projects="futureProjects"
      :reset-project="resetProject"
    />
  </div>
</template>
<script setup>
import {
  ScaleIcon,
  AlignLeftIcon as DescriptionIcon,
  BookmarkIcon,
  ChartIcon,
  CheckIcon,
  ClipboardCopyIcon,
  CopyrightIcon,
  DownloadIcon,
  ServerIcon,
  ExternalIcon,
  GameIcon,
  HeartIcon,
  ImageIcon as GalleryIcon,
  InfoIcon,
  LinkIcon as LinksIcon,
  MoreVerticalIcon,
  PlusIcon,
  ReportIcon,
  SearchIcon,
  SettingsIcon,
  TagsIcon,
  UsersIcon,
  VersionIcon,
  WrenchIcon,
  LanguagesIcon as TranslateIcon,
  ClientIcon,
  BookTextIcon,
  MonitorSmartphoneIcon,
  WikiIcon,
  CalendarIcon,
  KoFiIcon,
  IssuesIcon,
  UserIcon,
  PayPalIcon,
  BiliBiliIcon,
  SpigotMcIcon,
  QuarkIcon,
  BaiduIcon,
  CurseforgeIcon,
  McmodIcon,
  Mc9yIcon,
  ModrinthIcon2,
  AifadianIcon,
  QQPDIcon,
  WebIcon2,
  OopzIcon,
  KookIcon,
  CrownIcon,
  OpenCollectiveIcon,
  CodeIcon,
  CurrencyIcon,
  XIcon,
} from "@modrinth/assets";
import {
  Avatar,
  ButtonStyled,
  Checkbox,
  NewModal,
  OverflowMenu,
  PopoutMenu,
  ScrollablePanel,
  ContentPageHeader,
  provideProjectPageContext,
} from "@modrinth/ui";
import {
  formatCategory,
  isRejected,
  isStaff,
  isUnderReview,
  renderString,
  formatDateTime,
} from "@modrinth/utils";
import dayjs from "dayjs";
import Badge from "~/components/ui/Badge.vue";
import NavTabs from "~/components/ui/NavTabs.vue";
import NavStack from "~/components/ui/NavStack.vue";
import NavStackItem from "~/components/ui/NavStackItem.vue";
import ProjectMemberHeader from "~/components/ui/ProjectMemberHeader.vue";
import MessageBanner from "~/components/ui/MessageBanner.vue";
import { reportProject } from "~/utils/report-helpers.ts";
import Breadcrumbs from "~/components/ui/Breadcrumbs.vue";
import { userCollectProject } from "~/composables/user.js";
import CollectionCreateModal from "~/components/ui/CollectionCreateModal.vue";
import ModerationChecklist from "~/components/ui/ModerationChecklist.vue";
import Accordion from "~/components/ui/Accordion.vue";
import VersionSummary from "~/components/ui/VersionSummary.vue";
import AutomaticAccordion from "~/components/ui/AutomaticAccordion.vue";
import TranslationPromo from "~/components/ui/TranslationPromo.vue";
import ServerPromo from "~/components/ui/ServerPromo.vue";
import PurchaseButton from "~/components/ui/PurchaseButton.vue";
import { getVersionsToDisplay } from "~/helpers/projects.js";
import { projectAffiliates } from "~/config/affiliates.ts";
const data = useNuxtApp();
const route = useNativeRoute();

// Remove main padding for immersive hero effect
onMounted(() => {
  const mainEl = document.querySelector("main");
  if (mainEl) {
    mainEl.style.paddingTop = "0";
  }
});

onUnmounted(() => {
  const mainEl = document.querySelector("main");
  if (mainEl) {
    mainEl.style.paddingTop = "";
  }
});

const auth = await useAuth();
const user = await useUser();

const tags = useTags();

const { formatMessage } = useVIntl();

const settingsModal = ref();
const downloadModal = ref();
const createWikiModal = ref();
const preReviewWiki = ref();
const overTheTopDownloadAnimation = ref();

const userSelectedGameVersion = ref(null);
const userSelectedPlatform = ref(null);
const showAllVersions = ref(false);

const gameVersionFilterInput = ref();
const translationRecommendation = ref(null);

const versionFilter = ref("");

const createWikiTitle = ref("");
const createWikiSlug = ref("");
const createWikiSort = ref(0);
const createWikiFather = ref(null);

const currentGameVersion = computed(() => {
  if (!project || !project.value) return userSelectedGameVersion.value;
  return (
    userSelectedGameVersion.value ||
    (project.value.game_versions.length === 1 && project.value.game_versions[0])
  );
});

// possibleGameVersions 和 possiblePlatforms 将在 versions 初始化后定义

const currentPlatform = computed(() => {
  if (!project || !project.value) return userSelectedPlatform.value;
  return (
    userSelectedPlatform.value || (project.value.loaders.length === 1 && project.value.loaders[0])
  );
});
const gameVersionAccordion = ref();
const platformAccordion = ref();
const WikiFatherAccordion = ref();

// 组织级别的联机搭建配置
const ORG_AFFILIATES = {
  K1ZYxGDQ: "LaotouY",
  "6FNyvmc5": "LaotouY",
};

// 获取项目的联机搭建 affiliate key
const projectAffKey = computed(() => {
  if (!project.value) return null;

  // 首先检查项目级别的配置
  const projectAff = projectAffiliates[project.value.id];
  if (projectAff) return projectAff;

  // 然后检查组织级别的配置
  if (organization?.value?.id) {
    const orgAff = ORG_AFFILIATES[organization.value.id];
    if (orgAff) return orgAff;
  }

  return null;
});
const compatibilityMessages = defineMessages({
  title: {
    id: "project.about.compatibility.title",
    defaultMessage: "基本信息",
  },
  minecraftJava: {
    id: "project.about.compatibility.game.minecraftJava",
    defaultMessage: "我的世界Java版本",
  },
  platforms: {
    id: "project.about.compatibility.platforms",
    defaultMessage: "平台",
  },
  environments: {
    id: "project.about.compatibility.environments",
    defaultMessage: "运行环境",
  },
});
defineMessages({
  title: {
    id: "project.about.links.title",
    defaultMessage: "链接",
  },
  issues: {
    id: "project.about.links.issues",
    defaultMessage: "反馈问题",
  },
  source: {
    id: "project.about.links.source",
    defaultMessage: "查看源码",
  },
  wiki: {
    id: "project.about.links.wiki",
    defaultMessage: "访问 wiki",
  },
  discord: {
    id: "project.about.links.discord",
    defaultMessage: "加入discord服务器",
  },
  donateGeneric: {
    id: "project.about.links.donate.generic",
    defaultMessage: "捐赠",
  },
  donateGitHub: {
    id: "project.about.links.donate.github",
    defaultMessage: "Github赞助商",
  },
  donateBmac: {
    id: "project.about.links.donate.bmac",
    defaultMessage: "为我买一杯咖啡",
  },
  donatePatreon: {
    id: "project.about.links.donate.patreon",
    defaultMessage: "在 Patreon 上捐赠",
  },
  donatePayPal: {
    id: "project.about.links.donate.paypal",
    defaultMessage: "在 PayPal 上捐赠",
  },
  donateKoFi: {
    id: "project.about.links.donate.kofi",
    defaultMessage: "在 Ko-fi 上捐赠",
  },
  donateGithub: {
    id: "project.about.links.donate.github",
    defaultMessage: "在 Github 上捐赠",
  },
});
const creatorsMessages = defineMessages({
  title: {
    id: "project.about.creators.title",
    defaultMessage: "创作者",
  },
  owner: {
    id: "project.about.creators.owner",
    defaultMessage: "负责人",
  },
});
const detailsMessages = defineMessages({
  title: {
    id: "project.about.details.title",
    defaultMessage: "详情信息",
  },
  licensed: {
    id: "project.about.details.licensed",
    defaultMessage: "许可证 {license}",
  },
  created: {
    id: "project.about.details.created",
    defaultMessage: "创建于 {date}",
  },
  submitted: {
    id: "project.about.details.submitted",
    defaultMessage: "提交于 {date}",
  },
  published: {
    id: "project.about.details.published",
    defaultMessage: "发布于 {date}",
  },
  updated: {
    id: "project.about.details.updated",
    defaultMessage: "更新于 {date}",
  },
});

const modalLicense = ref(null);
const licenseText = ref("");

const webDisplayLabel = (x) => {
  switch (x) {
    case "other":
      return "其他";
    case "site":
      return "发布地址";

    case "modrinth":
      return "Modrinth";

    case "bilibili":
      return "哔哩哔哩";

    case "pd-qq":
      return "QQ频道";

    case "oopz":
      return "Oopz频道";

    case "kook":
      return "KOOK频道";

    case "afdian":
      return "爱发电";

    case "spigotmc":
      return "水龙头";

    case "curseforge":
      return "CurseForge地址";
    case "quark":
      return "夸克网盘";
    case "baidu":
      return "百度网盘";
    case "mcmod":
      return "MC百科";
    case "mc9y":
      return "九域资源社区";
    default:
      return x;
  }
};

const fromNow = (date) => {
  const currentDate = useCurrentDate();
  return dayjs(date).from(currentDate.value);
};

const formatDate = (date) => {
  return dayjs(date).format("YYYY-MM-DD");
};

const licenseIdDisplay = computed(() => {
  if (!project || !project.value) return "";
  const id = project.value.license.id;

  if (id === "LicenseRef-All-Rights-Reserved") {
    return "保留所有权益/无许可证";
  } else if (id.includes("LicenseRef")) {
    return id.replaceAll("LicenseRef-", "").replaceAll("-", " ");
  } else {
    return id;
  }
});

async function getLicenseData(event) {
  modalLicense.value.show(event);

  try {
    const text = await useBaseFetch(`tag/license/${project.value.license.id}`);
    licenseText.value = text.body || "无法检索许可证文本.";
  } catch {
    licenseText.value = "无法检索许可证文本.";
  }
}

// filteredVersions 和相关 computed 将在 versions 初始化后定义

const messages = defineMessages({
  downloadsStat: {
    id: "project.stats.downloads-label",
    defaultMessage: "下载{count, plural, one {} other {s}}",
  },
  followersStat: {
    id: "project.stats.followers-label",
    defaultMessage: "关注者{count, plural, one {} other {s}}",
  },
  descriptionTab: {
    id: "project.description.title",
    defaultMessage: "简介",
  },
  galleryTab: {
    id: "project.gallery.title",
    defaultMessage: "渲染图",
  },
  versionsTab: {
    id: "project.versions.title",
    defaultMessage: "版本",
  },
  moderationTab: {
    id: "project.moderation.title",
    defaultMessage: "管理",
  },
});

const displayCollectionsSearch = ref("");
const collections = computed(() =>
  user.value && user.value.collections
    ? user.value.collections.filter((x) =>
        x.name.toLowerCase().includes(displayCollectionsSearch.value.toLowerCase()),
      )
    : [],
);

if (
  !route.params.id ||
  !(
    tags.value.projectTypes.find((x) => x.id === route.params.type) ||
    route.params.type === "project"
  )
) {
  throw createError({
    fatal: true,
    statusCode: 404,
    message: "找不到该页面",
  });
}

let project,
  resetProject,
  allMembers,
  resetMembers,
  dependencies,
  featuredVersions,
  versions,
  wikis,
  organization,
  resetOrganization,
  translationPackProject,
  translationSourceProject;

/**
 * 检查错误消息是否为网络相关错误
 * @param {string} message - 错误消息
 * @returns {boolean}
 */
const isNetworkError = (message) => {
  const networkErrorPatterns = [
    "Failed to fetch",
    "CORS",
    "NetworkError",
    "timeout",
    "ECONNREFUSED",
    "ENOTFOUND",
    "ETIMEDOUT",
    "Network request failed",
  ];
  return networkErrorPatterns.some((pattern) => message.includes(pattern));
};

/**
 * 处理 API 错误并抛出适当的 createError
 * @param {Error} error - 错误对象
 * @throws {H3Error} - 抛出格式化的错误
 */
const handleApiError = (error) => {
  const statusCode =
    error?.statusCode || error?.status || error?.response?.status || error?.data?.statusCode || 404;
  const errorData = error?.data;
  const errorType = errorData?.error;
  const message = error?.message || "";

  // 优先检查限流错误
  if (statusCode === 429 || errorType === "ratelimit_error") {
    throw createError({
      fatal: true,
      statusCode: 429,
      message: errorData?.description || "请求过于频繁",
    });
  }

  // 检查网络错误（CORS、连接失败、超时等）
  if (isNetworkError(message)) {
    throw createError({
      fatal: true,
      statusCode: 503,
      message: "服务暂时不可用，请稍后重试",
    });
  }

  // 其他错误使用原状态码
  throw createError({
    fatal: true,
    statusCode,
    message: errorData?.description || error?.statusMessage || message || "资源不存在",
  });
};

try {
  const results = await Promise.allSettled([
    useAsyncData(`project/${route.params.id}`, () => useBaseFetch(`project/${route.params.id}`), {
      transform: (project) => {
        if (project) {
          project.actualProjectType = JSON.parse(JSON.stringify(project.project_type));
          project.project_type = data.$getProjectTypeForUrl(
            project.project_type,
            project.loaders,
            tags.value,
          );
        }

        return project;
      },
    }),
    useAsyncData(
      `project/${route.params.id}/members`,
      () => useBaseFetch(`project/${route.params.id}/members`, { apiVersion: 3 }),
      {
        transform: (members) => {
          members.forEach((it, index) => {
            members[index].avatar_url = it.user.avatar_url;
            members[index].name = it.user.username;
          });

          return members;
        },
      },
    ),
    useAsyncData(`project/${route.params.id}/dependencies`, () =>
      useBaseFetch(`project/${route.params.id}/dependencies`, {}),
    ),
    useAsyncData(`project/${route.params.id}/version?featured=true`, () =>
      useBaseFetch(`project/${route.params.id}/version?featured=true`),
    ),
    useAsyncData(`project/${route.params.id}/version`, () =>
      useBaseFetch(`project/${route.params.id}/version`),
    ),
    useAsyncData(`project/${route.params.id}/wiki`, () => {
      return useBaseFetch(`project/${route.params.id}/wiki`, { apiVersion: 3 });
    }),
    useAsyncData(`project/${route.params.id}/organization`, () =>
      useBaseFetch(`project/${route.params.id}/organization`, { apiVersion: 3 }),
    ),
  ]);

  /**
   * 从 Promise.allSettled 结果中提取数据
   * @param {PromiseSettledResult} result - Promise.allSettled 的单个结果
   * @param {boolean} needRefresh - 是否需要返回 refresh 函数
   * @returns {{ data: Ref, refresh?: Function, error?: Ref }}
   */
  const extractResult = (result, needRefresh = false) => {
    if (result.status === "fulfilled") {
      return result.value;
    }
    return needRefresh
      ? { data: ref(null), refresh: () => {}, error: ref(result.reason) }
      : { data: ref(null), error: ref(result.reason) };
  };

  const [
    projectResult,
    membersResult,
    dependenciesResult,
    featuredVersionsResult,
    versionsResult,
    wikisResult,
    organizationResult,
  ] = results;

  // 提取各个请求的结果
  let projectError;
  ({
    data: project,
    refresh: resetProject,
    error: projectError,
  } = extractResult(projectResult, true));
  ({ data: allMembers, refresh: resetMembers } = extractResult(membersResult, true));
  ({ data: dependencies } = extractResult(dependenciesResult));
  ({ data: featuredVersions } = extractResult(featuredVersionsResult));
  ({ data: versions } = extractResult(versionsResult));
  ({ data: wikis } = extractResult(wikisResult));
  ({ data: organization, refresh: resetOrganization } = extractResult(organizationResult, true));

  // 只检查主要资源（project）的错误
  // organization、wiki 等返回 404 是正常的，不应触发错误页面
  /**
   * 获取错误值，处理 ref 和普通值两种情况
   * @param {Ref|Error|null} err - 可能是 ref 或普通错误对象
   * @returns {Error|null}
   */
  const getErrorValue = (err) => (err?.value !== undefined ? err.value : err);
  const mainError = getErrorValue(projectError);

  if (mainError) {
    handleApiError(mainError);
  }

  versions = shallowRef(toRaw(versions));
  featuredVersions = shallowRef(toRaw(featuredVersions));

  // 如果项目启用了汉化追踪且有 translation_tracker，获取汉化包项目信息
  if (project.value && project.value.translation_tracker) {
    try {
      const { data: packProject } = await useAsyncData(
        `project/${project.value.translation_tracker}`,
        () => useBaseFetch(`project/${project.value.translation_tracker}`),
      );
      translationPackProject = packProject;
    } catch {
      // 汉化包项目不存在或获取失败，忽略错误
      translationPackProject = ref(null);
    }
  } else {
    translationPackProject = ref(null);
  }

  // 如果项目有 translation_source，获取原始项目信息（当前项目是汉化包）
  if (project.value && project.value.translation_source) {
    try {
      const { data: sourceProject } = await useAsyncData(
        `project/${project.value.translation_source}`,
        () => useBaseFetch(`project/${project.value.translation_source}`),
      );
      translationSourceProject = sourceProject;
    } catch {
      // 原始项目不存在或获取失败，忽略错误
      translationSourceProject = ref(null);
    }
  } else {
    translationSourceProject = ref(null);
  }
} catch (err) {
  // 使用通用错误处理函数
  handleApiError(err);
}

// 提供 ProjectPageContext 供子组件使用
const refreshVersions = async () => {
  const newVersions = await useBaseFetch(`project/${route.params.id}/version`);
  if (versions && versions.value) {
    versions.value = newVersions;
  }
};

const projectV2 = computed(() => project.value);

provideProjectPageContext({
  projectV2,
  refreshVersions,
});

// 在 versions 初始化后定义依赖它的 computed 属性
const possibleGameVersions = computed(() => {
  if (!versions || !versions.value) return [];
  return versions.value
    .filter((x) => !currentPlatform.value || x.loaders.includes(currentPlatform.value))
    .flatMap((x) => x.game_versions);
});

const possiblePlatforms = computed(() => {
  if (!versions || !versions.value) return [];
  return versions.value
    .filter((x) => !currentGameVersion.value || x.game_versions.includes(currentGameVersion.value))
    .flatMap((x) => x.loaders);
});

const filteredVersions = computed(() => {
  if (!versions || !versions.value) return [];
  return versions.value.filter(
    (x) =>
      (x.game_versions.length === 0 || x.game_versions.includes(currentGameVersion.value)) &&
      (x.loaders.includes(currentPlatform.value) || project.value.project_type === "resourcepack"),
  );
});

const filteredRelease = computed(() => {
  return filteredVersions.value.find((x) => x.version_type === "release");
});

const filteredBeta = computed(() => {
  return filteredVersions.value.find(
    (x) =>
      x.version_type === "beta" &&
      (!filteredRelease.value ||
        dayjs(x.date_published).isAfter(dayjs(filteredRelease.value.date_published))),
  );
});

const filteredAlpha = computed(() => {
  return filteredVersions.value.find(
    (x) =>
      x.version_type === "alpha" &&
      (!filteredRelease.value ||
        dayjs(x.date_published).isAfter(dayjs(filteredRelease.value.date_published))) &&
      (!filteredBeta.value ||
        dayjs(x.date_published).isAfter(dayjs(filteredBeta.value.date_published))),
  );
});

// 当过滤的版本改变时，更新汉化包推荐
watch([filteredRelease, filteredBeta, filteredAlpha], () => {
  if (downloadModal.value && downloadModal.value.isOpen) {
    fetchTranslationRecommendation();
  }
});

if (wikis.value && wikis.value.wikis) {
  wikis.value.wikis.sort((a, b) => a.sort_order - b.sort_order);
  wikis.value.wikis.forEach((wiki) => {
    wiki.child.sort((a, b) => a.sort_order - b.sort_order);
  });
}

if (!project.value) {
  throw createError({
    fatal: true,
    statusCode: 404,
    message: "资源不存在",
  });
}

if (project.value.project_type !== route.params.type || route.params.id !== project.value.slug) {
  let path = route.fullPath.split("/");
  path.splice(0, 3);
  path = path.filter((x) => x);

  await navigateTo(
    `/${project.value.project_type}/${project.value.slug}${
      path.length > 0 ? `/${path.join("/")}` : ""
    }`,
    { redirectCode: 301, replace: true },
  );
}

// 成员应为所有成员的数组，不包括已接受的成员，并以具有所有者角色的用户开头
// 其余成员应按角色排序，然后按姓名排序
const members = computed(() => {
  if (!allMembers || !allMembers.value) return [];
  const acceptedMembers = allMembers.value.filter((x) => x.accepted);
  const owner = acceptedMembers.find((x) =>
    organization?.value
      ? organization.value.members.some(
          (orgMember) => orgMember.user.id === x.user.id && orgMember.is_owner,
        )
      : x.is_owner,
  );

  const rest = acceptedMembers.filter((x) => !owner || x.user.id !== owner.user.id) || [];

  rest.sort((a, b) => {
    if (a.role === b.role) {
      return a.user.username.localeCompare(b.user.username);
    } else {
      return a.role.localeCompare(b.role);
    }
  });

  return owner ? [owner, ...rest] : rest;
});

const currentMember = computed(() => {
  let val =
    auth.value.user && allMembers?.value
      ? allMembers.value.find((x) => x.user.id === auth.value.user.id)
      : null;

  if (!val && auth.value.user && organization.value && organization.value.members) {
    val = organization.value.members.find((x) => x.user.id === auth.value.user.id);
  }

  if (!val && auth.value.user && tags.value.staffRoles.includes(auth.value.user.role)) {
    val = {
      team_id: project.team_id,
      user: auth.value.user,
      role: auth.value.role,
      permissions: auth.value.user.role === "admin" ? 2047 : 12,
      accepted: true,
      payouts_split: 0,
      avatar_url: auth.value.user.avatar_url,
      name: auth.value.user.username,
    };
  }

  return val;
});

// 上游修复: 防止 labrinth 下线时页面崩溃
versions.value = data.$computeVersions(versions.value ?? [], allMembers.value);

// 问：为什么要这样做，而不是计算 featuredVersions 的版本？
// 答：它会错误地生成版本 slug，因为它没有所有版本的完整上下文。例如，如果 Forge 的版本 1.1.0 是特色版本，
// 但 Fabric 的版本 1.1.0 不是，但 Fabric 版本先上传，则 Forge 版本将链接到 Fabric 版本
const featuredIds = (featuredVersions.value ?? []).map((x) => x.id);
featuredVersions.value = (versions.value ?? []).filter((version) =>
  featuredIds.includes(version.id),
);

featuredVersions.value.sort((a, b) => {
  const aLatest = a.game_versions[a.game_versions.length - 1];
  const bLatest = b.game_versions[b.game_versions.length - 1];
  const gameVersions = tags.value.gameVersions.map((e) => e.version);
  return gameVersions.indexOf(aLatest) - gameVersions.indexOf(bLatest);
});

const projectTypeDisplay = computed(() => {
  if (!project || !project.value) return "";
  return data.$formatProjectType(
    data.$getProjectTypeForDisplay(project.value.project_type, project.value.loaders),
  );
});

const following = computed(
  () =>
    user.value &&
    user.value.follows &&
    project?.value &&
    user.value.follows.find((x) => x.id === project.value.id),
);

// 判断是否有访问权限（已购买/团队成员/管理员）
const hasAccessToProject = computed(() => {
  // 后端返回的购买状态（已包含团队成员和管理员判断）
  if (project.value?.user_has_purchased === true) return true;
  // 前端额外检查 currentMember（处理 SSR 和缓存情况）
  if (currentMember.value) return true;
  return false;
});

// 判断是否可以下载：非付费资源或有访问权限的付费资源
const canDownload = computed(() => {
  if (!project?.value) return false;
  // 非付费资源可以下载
  if (!project.value.is_paid) return true;
  // 付费资源需要有访问权限
  return hasAccessToProject.value;
});

// 判断是否需要购买（付费资源且无访问权限）
const needsPurchase = computed(() => {
  if (!project?.value) return false;
  return project.value.is_paid && !hasAccessToProject.value;
});

const title = computed(() => {
  if (!project || !project.value) return "";
  return `${project.value.title} - 我的世界${projectTypeDisplay.value} | BBSMC 下载`;
});
const description = computed(() => {
  if (!project || !project.value) return "";
  const owner = members.value.find((x) => x.is_owner)?.user?.username || "创作者";
  const desc = project.value.description?.trim();
  return desc
    ? `${desc} - 在 BBSMC 下载我的世界${projectTypeDisplay.value} ${project.value.title}，由 ${owner} 创建。`
    : `在 BBSMC 下载我的世界${projectTypeDisplay.value} ${project.value.title}，由 ${owner} 创建。浏览详情、版本列表和社区评价。`;
});

if (!route.name?.startsWith("type-id-settings")) {
  useSeoMeta({
    title: () => title.value,
    description: () => description.value,
    ogTitle: () => title.value,
    ogDescription: () => description.value,
    ogImage: () => project.value.icon_url ?? "https://cdn.bbsmc.net/raw/placeholder.png",
    robots: () =>
      project.value.status === "approved" || project.value.status === "archived"
        ? "all"
        : "noindex",
  });

  useHead({
    script: [
      {
        type: "application/ld+json",
        children: () =>
          JSON.stringify({
            "@context": "https://schema.org",
            "@type": "SoftwareApplication",
            name: project.value.title,
            description: project.value.description,
            image: project.value.icon_url || undefined,
            url: `https://bbsmc.net/${route.params.type}/${project.value.slug || project.value.id}`,
            applicationCategory: "GameApplication",
            operatingSystem: "Windows, macOS, Linux",
            author: {
              "@type": "Person",
              name: members.value.find((x) => x.is_owner)?.user?.username || "",
            },
            interactionStatistic: [
              {
                "@type": "InteractionCounter",
                interactionType: "https://schema.org/DownloadAction",
                userInteractionCount: project.value.downloads,
              },
              {
                "@type": "InteractionCounter",
                interactionType: "https://schema.org/FollowAction",
                userInteractionCount: project.value.followers,
              },
            ],
            datePublished: project.value.published,
            dateModified: project.value.updated,
          }),
      },
    ],
  });
}

const onUserCollectProject = useClientTry(userCollectProject);

async function setProcessing() {
  startLoading();

  try {
    await useBaseFetch(`project/${project.value.id}`, {
      method: "PATCH",
      body: {
        status: "processing",
      },
    });

    project.value.status = "processing";
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }

  stopLoading();
}

async function patchProject(resData, quiet = false) {
  let result = false;
  startLoading();

  try {
    await useBaseFetch(`project/${project.value.id}`, {
      method: "PATCH",
      body: resData,
    });

    for (const key in resData) {
      project.value[key] = resData[key];
    }

    if ("license_id" in resData) {
      project.value.license.id = resData.license_id;
    }
    if ("license_url" in resData) {
      project.value.license.url = resData.license_url;
    }

    result = true;
    if (!quiet) {
      data.$notify({
        group: "main",
        title: "资源已更新",
        text: "您的资源已更新",
        type: "success",
      });
      window.scrollTo({ top: 0, behavior: "smooth" });
    }
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  stopLoading();

  return result;
}

async function patchIcon(icon) {
  let result = false;
  startLoading();

  try {
    await useBaseFetch(
      `project/${project.value.id}/icon?ext=${
        icon.type.split("/")[icon.type.split("/").length - 1]
      }`,
      {
        method: "PATCH",
        body: icon,
      },
    );
    await resetProject();
    result = true;
    data.$notify({
      group: "main",
      title: "资源图标更新",
      text: "您的资源图标已更新",
      type: "success",
    });
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });

    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  stopLoading();
  return result;
}

async function updateMembers() {
  allMembers.value = await useAsyncData(
    `project/${route.params.id}/members`,
    () => useBaseFetch(`project/${route.params.id}/members`),
    {
      transform: (members) => {
        members.forEach((it, index) => {
          members[index].avatar_url = it.user.avatar_url;
          members[index].name = it.user.username;
        });

        return members;
      },
    },
  );
}

async function createWiki() {
  if (createWikiTitle.value === "" || createWikiSlug.value === "") {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "</br>请填写完整数据",
      type: "error",
    });
    return;
  }

  const resData = {
    title: createWikiTitle.value,
    slug: createWikiSlug.value,
    sort_order: createWikiSort.value,
  };

  if (createWikiFather.value) {
    resData.father_id = createWikiFather.value.id;
  }

  for (const wiki of wikis.value.cache.cache) {
    if (wiki.slug === resData.slug) {
      data.$notify({
        group: "main",
        title: "发生错误",
        text: "</br>已存在相同slug",
        type: "error",
      });
    }
    if (wiki.title === resData.title) {
      data.$notify({
        group: "main",
        title: "发生错误",
        text: "</br>已存在相同标题",
        type: "error",
      });
    }
  }

  const { data: newWiki } = await useAsyncData(`project/${route.params.id}/wiki_create`, () =>
    useBaseFetch(`project/${route.params.id}/wiki_create`, {
      apiVersion: 3,
      method: "POST",
      body: resData,
    }),
  );
  wikis.value.cache.cache.push(newWiki);
  wikis.value = newWiki.value;
  createWikiModal.value.hide();
  createWikiFather.value = null;
  createWikiTitle.value = "";
  createWikiSort.value = 0;
  createWikiSlug.value = "";
}

const preSortReview = ref([]);
const preBodyReview = ref([]);
const preADDReview = ref([]);
const preREMOVEReview = ref([]);
const preIndexSetReview = ref([]);
function submitForReview() {
  // 开始计算和之前有哪些变动

  submitWikiCacheMsg.value = "";
  // 第一步，获取到所有的新增的和被移除的WIKI
  const wikiNew = wikis.value.cache.cache;
  const wikiOld = wikis.value.wikis;

  wikiNew.forEach((wiki) => {
    if (!wiki.child) {
      wiki.child = [];
    }
  });
  wikiOld.forEach((wiki) => {
    if (!wiki.child) {
      wiki.child = [];
    }
  });

  const wikiNews = wikiNew.flatMap((x) => [x, ...x.child.map((child) => child)]);
  const wikiOlds = wikiOld.flatMap((x) => [x, ...x.child.map((child) => child)]);
  const wikiNewId = wikiNew.flatMap((x) => [x.id, ...x.child.map((child) => child.id)]);
  const wikiOldId = wikiOld.flatMap((x) => [x.id, ...x.child.map((child) => child.id)]);

  const commonIds = wikiNewId.filter((id) => wikiOldId.includes(id)); // 交集
  const addWiki = [...wikiNews].filter((wiki) => !wikiOldId.includes(wiki.id));
  const removedWiki = [...wikiOlds].filter((wiki) => !wikiNewId.includes(wiki.id));

  const commonObjects = new Map(
    wikiNews.map((wiki) => [wiki.id, [wikiOlds.find((oldWiki) => oldWiki.id === wiki.id), wiki]]),
  );

  preSortReview.value = [];
  preBodyReview.value = [];
  preADDReview.value = [];
  preREMOVEReview.value = [];
  preIndexSetReview.value = [];
  let featured = null;
  wikiOlds.forEach((wiki) => {
    if (wiki.featured) {
      featured = wiki;
    } else if (wiki.child && wiki.child.length > 0) {
      wiki.child.forEach((wiki__) => {
        if (wiki__.featured) {
          featured = wiki__;
        }
      });
    }
  });
  wikiNews.forEach((wiki) => {
    if (wiki.featured) {
      if (!featured) {
        preIndexSetReview.value.push({
          wiki_id: wiki.id,
          title: wiki.title,
        });
      } else if (featured && featured.id !== wiki.id) {
        preIndexSetReview.value.push({
          wiki_id: wiki.id,
          title: wiki.title,
        });
      }
    } else if (wiki.child && wiki.child.length > 0) {
      wiki.child.forEach((wiki__) => {
        if (wiki__.featured) {
          if (!featured) {
            preIndexSetReview.value.push({
              wiki_id: wiki__.id,
              title: wiki__.title,
            });
          } else if (featured && featured.id !== wiki__.id) {
            preIndexSetReview.value.push({
              wiki_id: wiki__.id,
              title: wiki__.title,
            });
          }
        }
      });
    }
  });

  commonObjects.forEach(([oldWiki, newWiki]) => {
    if (oldWiki && commonIds.includes(oldWiki.id)) {
      if (oldWiki.sort_order !== newWiki.sort_order) {
        preSortReview.value.push({
          wiki_id: oldWiki.id,
          title: newWiki.title,
          old_sort_order: oldWiki.sort_order,
          new_sort_order: newWiki.sort_order,
        });
      }
      if (oldWiki.body !== newWiki.body) {
        preBodyReview.value.push({
          wiki_id: oldWiki.id,
          title: newWiki.title,
          old_body: oldWiki.body,
          new_body: newWiki.body,
        });
      }
    }
  });
  addWiki.forEach((wiki) => {
    preADDReview.value.push({
      wiki_id: wiki.id,
      title: wiki.title,
      sort_order: wiki.sort_order,
      body: wiki.body,
    });
  });

  removedWiki.forEach((wiki) => {
    preREMOVEReview.value.push({
      wiki_id: wiki.id,
      title: wiki.title,
      sort_order: wiki.sort_order,
      body: wiki.body,
    });
  });

  if (
    preIndexSetReview.value.length +
      preBodyReview.value.length +
      preSortReview.value.length +
      preREMOVEReview.value.length +
      preADDReview.value.length ===
    0
  ) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "</br>没有任何修改",
      type: "error",
    });
    return;
  }
  preReviewWiki.value.show();

  // 第二部，获取所有被修改过的WIKI

  // 第三步， 获取所有修改了权重的WIKI
}
const router = useNativeRouter();
const submitWikiCacheMsg = ref("");

async function submitConfirmForReview() {
  if (submitWikiCacheMsg.value === "") {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "</br>请填写提交原因",
      type: "error",
    });
    return;
  }
  try {
    await useBaseFetch(`project/${route.params.id}/wiki_submit`, {
      apiVersion: 3,
      method: "POST",
      body: { msg: submitWikiCacheMsg.value },
    });
    data.$notify({
      group: "main",
      title: "成功",
      text: "</br>提交成功",
      type: "success",
    });
    preReviewWiki.value.hide();

    router.push(`/project/${route.params.id}/wikis`);
  } catch (err) {
    data.$notify({
      group: "main",
      title: "发生错误",
      text: err.data.description,
      type: "error",
    });
  }
}

async function copyId() {
  await navigator.clipboard.writeText(project.value.id);
}

const collapsedChecklist = ref(false);

const showModerationChecklist = ref(false);
const futureProjects = ref([]);
if (import.meta.client && history && history.state && history.state.showChecklist) {
  showModerationChecklist.value = true;
  futureProjects.value = history.state.projects;
}

function closeDownloadModal(event) {
  downloadModal.value.hide(event);
  userSelectedPlatform.value = null;
  userSelectedGameVersion.value = null;
  showAllVersions.value = false;
}

function triggerDownloadAnimation() {
  overTheTopDownloadAnimation.value = true;
  setTimeout(() => (overTheTopDownloadAnimation.value = false), 500);
}

function onDownload(event) {
  triggerDownloadAnimation();
  setTimeout(() => {
    closeDownloadModal(event);
    if (event) {
      useBaseFetch(`version/${event}/download`, {
        method: "PATCH",
        apiVersion: 3,
      });
    }
  }, 400);
}

async function fetchTranslationRecommendation() {
  // 确保数据已经初始化
  if (!versions || !versions.value) {
    translationRecommendation.value = null;
    return;
  }

  // 获取当前显示的版本
  const targetVersion = filteredRelease.value || filteredBeta.value || filteredAlpha.value;

  if (!targetVersion || !targetVersion.translated_by || targetVersion.translated_by.length === 0) {
    translationRecommendation.value = null;
    return;
  }

  try {
    // 获取所有汉化版本的详细信息
    const translationData = await Promise.all(
      targetVersion.translated_by.map(async (translationLink) => {
        try {
          const translationVersionId = translationLink.joining_version_id;

          // 先获取版本信息
          const translationVersion = await useBaseFetch(`version/${translationVersionId}`);

          if (translationVersion && translationVersion.project_id) {
            // 再通过 project_id 获取项目信息
            const translationProject = await useBaseFetch(
              `project/${translationVersion.project_id}`,
            );

            if (translationProject) {
              return {
                version: translationVersion,
                project: translationProject,
                language_code: translationLink.language_code,
                description: translationLink.description,
                date_published: translationVersion.date_published,
              };
            }
          }
        } catch (error) {
          console.error("获取单个汉化包失败:", error);
          return null;
        }
      }),
    );

    // 过滤掉获取失败的项
    const validTranslations = translationData.filter((item) => item !== null);

    // 按项目ID分组，每个项目只保留最新的版本
    const translationsByProject = new Map();

    validTranslations.forEach((translation) => {
      const projectId = translation.project.id;
      const existing = translationsByProject.get(projectId);

      // 如果这个项目还没有记录，或者当前版本更新，则更新记录
      if (!existing || new Date(translation.date_published) > new Date(existing.date_published)) {
        translationsByProject.set(projectId, translation);
      }
    });

    // 转换回数组并按发布时间排序（最新的在前）
    const uniqueTranslations = Array.from(translationsByProject.values()).sort(
      (a, b) => new Date(b.date_published) - new Date(a.date_published),
    );

    if (uniqueTranslations.length > 0) {
      translationRecommendation.value = uniqueTranslations;
    } else {
      translationRecommendation.value = null;
    }
  } catch (error) {
    translationRecommendation.value = null;
  }
}

function navigateToTranslation(translationData) {
  if (translationData && translationData.project && translationData.version) {
    const projectType = translationData.project.project_type;
    const projectId = translationData.project.slug || translationData.project.id;
    const versionId = translationData.version.id;
    // 跳转到汉化包的具体版本页面
    navigateTo(`/${projectType}/${projectId}/version/${versionId}`);
    downloadModal.value.hide();
  }
}

function navigateToServer() {
  // 跳转到服务器页面，与联机搭建按钮的跳转逻辑一致
  const affId = projectAffKey.value;
  if (affId === "pcl") {
    window.open("/pcl", "_blank");
  } else if (affId) {
    window.open(`/server?aff=${affId}`, "_blank");
  }
  downloadModal.value.hide();
}

function onDownloadClick(event) {
  if (!project || !project.value) {
    return;
  }

  if (project.value.versions.length === 0) {
    for (const url of project.value.donation_urls) {
      if (url.id === "site") {
        window.open(url.url, "_blank");
        return;
      }
    }
    data.$notify({
      group: "main",
      title: "发生错误",
      text: "该资源没有可用下载源",
      type: "error",
    });
    return;
  }

  // 打开下载弹框时获取汉化包推荐
  fetchTranslationRecommendation();
  downloadModal.value.show(event);
}

// 处理购买按钮点击
function handlePurchaseClick() {
  // 未登录时跳转到登录页面
  if (!auth.value.user) {
    navigateTo("/auth/sign-in");
    return;
  }
  // 已登录时滚动到购买区域
  scrollToPurchase();
}

// 滚动到购买区域
function scrollToPurchase() {
  const purchaseCard = document.querySelector(".purchase-card");
  if (purchaseCard) {
    purchaseCard.scrollIntoView({ behavior: "smooth", block: "center" });
    // 添加高亮动画效果
    purchaseCard.classList.add("highlight-pulse");
    setTimeout(() => {
      purchaseCard.classList.remove("highlight-pulse");
    }, 2000);
  }
}

const navLinks = computed(() => {
  if (!project || !project.value) return [];
  const projectUrl = `/${project.value.project_type}/${project.value.slug ? project.value.slug : project.value.id}`;

  return [
    {
      label: formatMessage(messages.descriptionTab),
      href: projectUrl,
    },
    {
      label: formatMessage(messages.galleryTab),
      href: `${projectUrl}/gallery`,
      shown: project?.value?.gallery?.length > 0 || !!currentMember.value,
    },
    {
      label: "更新日志",
      href: `${projectUrl}/changelog`,
      shown: versions?.value?.length > 0,
    },
    {
      label: formatMessage(messages.versionsTab),
      href: `${projectUrl}/versions`,
      shown: versions?.value?.length > 0 || !!currentMember.value,
      subpages: [`${projectUrl}/version/`],
    },
    {
      label: "百科",
      href: `${projectUrl}/wikis`,
      subpages: [`${projectUrl}/wiki/`],
    },
    {
      label: "反馈",
      href: `${projectUrl}/issues`,
      subpages: [`${projectUrl}/issues/`],
    },
    {
      label: "讨论",
      href: `${projectUrl}/forum`,
      subpages: [`${projectUrl}/forum/`],
    },
    {
      label: formatMessage(messages.moderationTab),
      href: `${projectUrl}/moderation`,
      shown:
        !!currentMember.value &&
        (isRejected(project.value) || isUnderReview(project.value) || isStaff(auth.value.user)),
    },
  ];
});
</script>
<style lang="scss" scoped>
// ==========================================
// FLAME THEME - Project Detail Page
// ==========================================

// Settings Header (for settings page mode)
.settings-header {
  display: flex;
  flex-direction: row;
  gap: var(--spacing-card-sm);
  align-items: center;
  margin-bottom: var(--spacing-card-bg);

  .settings-header__icon {
    flex-shrink: 0;
  }

  .settings-header__text {
    h1 {
      font-size: var(--font-size-md);
      margin-top: 0;
      margin-bottom: var(--spacing-card-sm);
    }
  }
}

// Popout Menu Styles
.popout-checkbox {
  padding: var(--gap-sm) var(--gap-md);
  white-space: nowrap;

  &:hover {
    filter: brightness(0.95);
  }
}

.popout-heading {
  padding: var(--gap-sm) var(--gap-md);
  padding-bottom: 0;
  font-size: var(--font-size-nm);
  color: var(--color-secondary);
}

.collection-button {
  margin: var(--gap-sm) var(--gap-md);
  white-space: nowrap;
}

.menu-text {
  padding: 0 var(--gap-md);
  font-size: var(--font-size-nm);
  color: var(--color-secondary);
}

.menu-search {
  margin: var(--gap-sm) var(--gap-md);
  width: calc(100% - var(--gap-md) * 2);
}

.collections-list {
  max-height: 40rem;
  overflow-y: auto;
  background-color: var(--color-bg);
  border-radius: var(--radius-md);
  margin: var(--gap-sm) var(--gap-md);
  padding: var(--gap-sm);
}

.normal-page__info:empty {
  display: none;
}

:deep(.accordion-with-bg) {
  @apply rounded-2xl bg-bg p-2;
  --scrollable-pane-bg: var(--color-bg);
}

// Download Animation
.over-the-top-download-animation {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: flex;
  justify-content: center;
  align-items: center;
  pointer-events: none;
  scale: 0.5;
  transition: all 0.5s ease-out;
  opacity: 1;

  &.animation-hidden {
    scale: 0.8;
    opacity: 0;

    .animation-ring-1 {
      width: 25rem;
      height: 25rem;
    }

    .animation-ring-2 {
      width: 50rem;
      height: 50rem;
    }

    .animation-ring-3 {
      width: 100rem;
      height: 100rem;
    }
  }

  > div {
    position: relative;
    display: flex;
    justify-content: center;
    align-items: center;
    width: fit-content;
    height: fit-content;

    > * {
      position: absolute;
      scale: 1;
      transition: all 0.2s ease-out;
      width: 20rem;
      height: 20rem;
    }
  }
}

@media (hover: none) and (max-width: 767px) {
  .modrinth-app-section {
    display: none;
  }
}

// Translation Pack Card
.translation-pack-card {
  display: flex;
  align-items: center;
  gap: var(--spacing-card-sm);
  padding: var(--spacing-card-sm);
  margin-top: var(--spacing-card-sm);
  background-color: var(--color-bg);
  border-radius: var(--radius-md);
  text-decoration: none;
  transition: all 0.3s var(--ease-out, ease);

  &:hover {
    background-color: var(--accent-muted, rgba(241, 100, 54, 0.08));
    border-color: var(--flame, #f16436);
  }

  .translation-pack-info {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-card-xs);
    min-width: 0;

    .translation-pack-title {
      font-weight: bold;
      color: var(--color-text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .translation-pack-desc {
      font-size: var(--font-size-sm);
      color: var(--color-text-secondary);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
  }
}

// ==========================================
// FLAME THEME OVERRIDES
// ==========================================

// Page Header Enhancement
:deep(.normal-page__header) {
  position: relative;
  margin-bottom: 24px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--color-divider);

  // Status Badge Styling
  .status-badge {
    margin-left: 12px;
  }
}

// Enhanced ContentPageHeader
:deep(.content-page-header) {
  position: relative;

  // Large Icon with glow effect
  .icon,
  [class*="avatar"] {
    border-radius: 16px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
    transition:
      transform 0.3s var(--ease-out, ease),
      box-shadow 0.3s var(--ease-out, ease);

    &:hover {
      transform: scale(1.02);
      box-shadow: 0 12px 32px rgba(241, 100, 54, 0.2);
    }
  }

  // Title Enhancement
  h1,
  .title {
    font-family: var(--font-display, inherit);
    font-weight: 800;
    letter-spacing: -0.02em;
    color: var(--color-text-dark, var(--color-text));
  }

  // Summary/Description
  .summary,
  .description {
    color: var(--color-secondary);
    line-height: 1.7;
  }
}

// Stats Enhancement
:deep(.stats),
:deep([class*="stats"]) {
  .stat-icon,
  svg {
    color: var(--color-secondary);
    transition: color 0.2s ease;
  }

  &:hover .stat-icon,
  &:hover svg {
    color: var(--flame, #f16436);
  }
}

// Tag List Enhancement - Clean design
.tag-list {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.tag-list__item {
  display: inline-flex;
  align-items: center;
  padding: 4px 10px;
  background: transparent;
  color: var(--color-text);
  font-size: 0.8rem;
  font-weight: 500;
  border-radius: 6px;
  border: 1px solid var(--color-divider);
  transition: all 0.2s ease;

  svg {
    width: 14px;
    height: 14px;
    margin-right: 6px;
    color: var(--_color, var(--color-secondary));
  }

  // Platform tags with color - keep background style
  &[style*="--_color"] {
    background: color-mix(in srgb, var(--_color) 12%, transparent);
    border-color: color-mix(in srgb, var(--_color) 30%, transparent);
    color: var(--_color);

    &:hover {
      background: color-mix(in srgb, var(--_color) 20%, transparent);
      border-color: var(--_color);
    }
  }

  &:hover {
    border-color: var(--flame, #f16436);
    color: var(--flame, #f16436);
  }
}

// ==========================================
// SIDEBAR CARDS - FLAME THEME
// ==========================================

// Base Card Styling
:deep(.card.flex-card) {
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 16px;
  padding: 16px 18px;
  transition: all 0.3s var(--ease-out, ease);

  &:hover {
    border-color: rgba(241, 100, 54, 0.3);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.08);
  }

  // Card Titles
  h2 {
    font-family: var(--font-display, inherit);
    font-size: 0.95rem;
    font-weight: 700;
    color: var(--color-text-dark, var(--color-text));
    margin-bottom: 8px;
    padding-bottom: 6px;
    border-bottom: 1px solid var(--color-divider);
    display: flex;
    align-items: center;
    gap: 8px;

    &::before {
      content: "";
      width: 3px;
      height: 16px;
      background: linear-gradient(180deg, var(--flame, #f16436) 0%, #ff8a5c 100%);
      border-radius: 2px;
    }
  }

  // Card Section Spacing
  section {
    margin-bottom: 8px;

    &:last-child {
      margin-bottom: 0;
    }
  }

  // Section Subtitles
  h3 {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--color-secondary);
    margin: 0 0 6px 0;
    text-transform: uppercase;
    letter-spacing: 0.02em;
  }
}

// Details List Enhancement - Clean, no background
:deep(.details-list) {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

:deep(.details-list__item) {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 4px;
  background: transparent;
  border-radius: 8px;
  font-size: 0.875rem;
  color: var(--color-text);
  transition: all 0.2s ease;
  text-decoration: none;

  svg {
    width: 16px;
    height: 16px;
    color: var(--color-secondary);
    flex-shrink: 0;
    transition: color 0.2s ease;
  }

  &:hover {
    color: var(--flame, #f16436);

    svg {
      color: var(--flame, #f16436);
    }
  }
}

// Large variant (for members) - keep subtle background
:deep(.details-list__item--type-large) {
  padding: 10px 12px;
  background: var(--color-bg);
  border-radius: 10px;

  .rows {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;

    span:first-child {
      font-weight: 600;
      color: var(--color-text-dark, var(--color-text));
    }
  }

  &:hover {
    background: var(--accent-muted, rgba(241, 100, 54, 0.08));
  }
}

:deep(.details-list__item__text--style-secondary) {
  font-size: 0.8rem;
  color: var(--color-secondary);
}

// External Links Enhancement
:deep(.details-list__item) {
  a,
  .text-link {
    color: var(--flame, #f16436);
    text-decoration: none;
    transition: all 0.2s ease;

    &:hover {
      text-decoration: underline;
    }
  }

  .external-icon {
    width: 14px;
    height: 14px;
    opacity: 0.5;
  }
}

// ==========================================
// NAVIGATION TABS - FLAME THEME
// ==========================================

// NavTabs component now handles its own underline indicator
// Only apply basic styling overrides here
:deep(.nav-tabs-underline) {
  margin-bottom: 16px;
}

// ==========================================
// BUTTONS - FLAME THEME
// ==========================================

// Download Button Enhancement
:deep([class*="button"][class*="green"]),
:deep(.btn-green),
:deep(button[class*="green"]) {
  background: linear-gradient(135deg, var(--flame, #f16436) 0%, #ff8a5c 100%) !important;
  border: none !important;
  color: #fff !important;
  font-weight: 700;
  box-shadow: 0 4px 16px rgba(241, 100, 54, 0.35);
  transition: all 0.3s var(--ease-out, ease);

  &:hover {
    transform: translateY(-2px);
    box-shadow: 0 8px 24px rgba(241, 100, 54, 0.45);
  }

  &:active {
    transform: translateY(0);
  }
}

// Action Buttons
:deep([class*="button"][class*="circular"]),
:deep(.btn-circular) {
  border-radius: 12px;
  transition: all 0.25s var(--ease-out, ease);

  &:hover {
    border-color: var(--flame, #f16436);
    background: var(--accent-muted, rgba(241, 100, 54, 0.1));

    svg {
      color: var(--flame, #f16436);
    }
  }
}

// Follow Button (Heart)
:deep([class*="button"][class*="red"]) {
  &:hover {
    background: rgba(239, 68, 68, 0.1);
  }
}

// Purple Button (Server)
:deep([class*="button"][class*="purple"]) {
  color: var(--purple, #a855f7);
  border-color: var(--purple, #a855f7);

  &:hover {
    background: rgba(168, 85, 247, 0.1);
  }
}

// ==========================================
// WIKI SIDEBAR - FLAME THEME
// ==========================================

:deep(.wiki-sidebar),
:deep([class*="wiki"]) {
  .wiki-nav-item,
  a {
    padding: 10px 14px;
    border-radius: 10px;
    transition: all 0.2s ease;

    &:hover {
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }

    &.active,
    &[aria-current="page"] {
      background: var(--accent-muted, rgba(241, 100, 54, 0.12));
      color: var(--flame, #f16436);
      font-weight: 600;

      &::before {
        content: "";
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        width: 3px;
        height: 60%;
        background: var(--flame, #f16436);
        border-radius: 2px;
      }
    }
  }
}

// ==========================================
// COMPATIBILITY SECTION
// ==========================================

:deep(.compatibility-info),
:deep([class*="compatibility"]) {
  .game-version-tag,
  .platform-tag,
  .environment-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: var(--color-bg);
    border-radius: 8px;
    font-size: 0.8rem;
    font-weight: 500;
    color: var(--color-text);
    transition: all 0.2s ease;

    &:hover {
      background: var(--accent-muted, rgba(241, 100, 54, 0.1));
      color: var(--flame, #f16436);
    }
  }
}

// Version Display Enhancement
:deep(.version-display),
:deep([class*="version"]:not(.nav-tabs)) {
  .version-badge {
    background: rgba(45, 212, 191, 0.12);
    color: var(--teal, #2dd4bf);
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.75rem;
    font-weight: 600;
  }
}

// ==========================================
// RESPONSIVE DESIGN
// ==========================================

@media (max-width: 1200px) {
  :deep(.nav-tabs) {
    overflow-x: auto;
    scrollbar-width: none;
    -ms-overflow-style: none;

    &::-webkit-scrollbar {
      display: none;
    }
  }
}

@media (max-width: 768px) {
  :deep(.card.flex-card) {
    padding: 16px;
    border-radius: 14px;
  }

  :deep(.details-list__item) {
    padding: 10px 12px;
  }

  .tag-list__item {
    font-size: 0.7rem;
    padding: 3px 8px;
  }
}

// ==========================================
// DARK/LIGHT THEME ADAPTATIONS
// ==========================================

// These use CSS variables that automatically adapt to theme
:root {
  --flame: #f16436;
  --accent-muted: rgba(241, 100, 54, 0.1);
  --accent-glow: rgba(241, 100, 54, 0.3);
  --ease-out: cubic-bezier(0.16, 1, 0.3, 1);
  --ease-spring: cubic-bezier(0.34, 1.56, 0.64, 1);
}

// Light theme specific overrides
:global(.light-mode),
:global([data-theme="light"]) {
  --bg-card: #ffffff;
  --color-text-dark: #1a1a2e;
  --accent-muted: rgba(241, 100, 54, 0.08);
}

// Dark theme specific overrides
:global(.dark-mode),
:global([data-theme="dark"]) {
  --bg-card: #1e2128;
  --color-text-dark: #ffffff;
  --accent-muted: rgba(241, 100, 54, 0.12);
}

// ==========================================
// HOVER ANIMATIONS
// ==========================================

@keyframes gentle-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.7;
  }
}

@keyframes slide-up {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

// Apply entrance animations to cards
:deep(.card.flex-card) {
  animation: slide-up 0.4s var(--ease-out, ease) both;

  &:nth-child(1) {
    animation-delay: 0.05s;
  }
  &:nth-child(2) {
    animation-delay: 0.1s;
  }
  &:nth-child(3) {
    animation-delay: 0.15s;
  }
  &:nth-child(4) {
    animation-delay: 0.2s;
  }
  &:nth-child(5) {
    animation-delay: 0.25s;
  }
  &:nth-child(6) {
    animation-delay: 0.3s;
  }
}

// ==========================================
// DOWNLOAD MODAL - FLAME THEME
// ==========================================

:deep(.modal),
:deep([class*="modal"]) {
  // Modal backdrop
  .modal-backdrop {
    backdrop-filter: blur(8px);
    background: rgba(0, 0, 0, 0.6);
  }

  // Modal container
  .modal-container,
  .modal-content {
    background: var(--bg-card, var(--color-raised-bg));
    border: 1px solid var(--color-divider);
    border-radius: 20px;
    box-shadow: 0 24px 48px rgba(0, 0, 0, 0.2);
    overflow: hidden;
  }

  // Modal header
  .modal-header,
  [class*="modal-header"] {
    padding: 20px 24px;
    border-bottom: 1px solid var(--color-divider);
    background: linear-gradient(
      135deg,
      var(--accent-muted, rgba(241, 100, 54, 0.05)) 0%,
      transparent 100%
    );

    .icon {
      border-radius: 12px;
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    }
  }

  // Modal body
  .modal-body,
  [class*="modal-body"] {
    padding: 24px;
  }
}

// Accordion in Modal Enhancement
:deep(.accordion-with-bg) {
  background: var(--color-bg) !important;
  border: 1px solid var(--color-divider);
  border-radius: 14px !important;
  overflow: hidden;

  // Accordion Header
  [class*="accordion-header"],
  summary {
    padding: 14px 18px;
    font-weight: 600;
    display: flex;
    align-items: center;
    gap: 10px;
    transition: all 0.2s ease;

    svg {
      width: 20px;
      height: 20px;
      color: var(--flame, #f16436);
    }

    &:hover {
      background: var(--accent-muted, rgba(241, 100, 54, 0.06));
    }
  }

  // Accordion Content
  [class*="accordion-content"],
  .accordion-body {
    padding: 12px;
    border-top: 1px solid var(--color-divider);
    background: var(--color-bg);
  }
}

// Version/Platform Selection Buttons in Modal
:deep(.accordion-with-bg) {
  button,
  [class*="button"] {
    margin: 4px;
    padding: 10px 16px;
    border-radius: 10px;
    font-size: 0.875rem;
    font-weight: 500;
    transition: all 0.2s ease;

    &:hover:not(.looks-disabled) {
      background: var(--accent-muted, rgba(241, 100, 54, 0.1));
      color: var(--flame, #f16436);
    }

    // Selected state
    &[class*="brand"],
    &.selected {
      background: var(--flame, #f16436) !important;
      color: #fff !important;
    }

    // Disabled/incompatible state
    &.looks-disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
  }
}

// Version Summary Cards in Download Modal
:deep(.version-summary),
:deep([class*="version-summary"]) {
  background: var(--color-bg);
  border: 1px solid var(--color-divider);
  border-radius: 14px;
  padding: 16px;
  transition: all 0.25s var(--ease-out, ease);

  &:hover {
    border-color: var(--flame, #f16436);
    box-shadow: 0 4px 16px rgba(241, 100, 54, 0.15);
  }

  // Version type badge
  .version-type-badge,
  [class*="type-badge"] {
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.05em;

    &.release,
    &[class*="release"] {
      background: rgba(34, 197, 94, 0.15);
      color: #22c55e;
    }

    &.beta,
    &[class*="beta"] {
      background: rgba(249, 115, 22, 0.15);
      color: #f97316;
    }

    &.alpha,
    &[class*="alpha"] {
      background: rgba(239, 68, 68, 0.15);
      color: #ef4444;
    }
  }

  // Download button in version card
  .download-btn,
  [class*="download"] {
    background: linear-gradient(135deg, var(--flame, #f16436) 0%, #ff8a5c 100%);
    color: #fff;
    border: none;
    padding: 8px 16px;
    border-radius: 10px;
    font-weight: 600;
    transition: all 0.2s ease;

    &:hover {
      transform: scale(1.02);
      box-shadow: 0 4px 12px rgba(241, 100, 54, 0.4);
    }
  }
}

// Search Input in Modal
:deep(.iconified-input) {
  background: var(--color-bg);
  border: 1px solid var(--color-divider);
  border-radius: 12px;
  padding: 10px 14px;
  display: flex;
  align-items: center;
  gap: 10px;
  transition: all 0.2s ease;

  svg {
    width: 18px;
    height: 18px;
    color: var(--color-secondary);
  }

  input {
    background: transparent;
    border: none;
    outline: none;
    flex: 1;
    font-size: 0.9rem;
    color: var(--color-text);

    &::placeholder {
      color: var(--color-secondary);
    }
  }

  &:focus-within {
    border-color: var(--flame, #f16436);
    box-shadow: 0 0 0 3px rgba(241, 100, 54, 0.15);
  }
}

// Translation Promo Enhancement
:deep(.translation-promo),
:deep([class*="translation-promo"]) {
  background: linear-gradient(135deg, rgba(34, 197, 94, 0.08) 0%, rgba(34, 197, 94, 0.02) 100%);
  border: 1px solid rgba(34, 197, 94, 0.2);
  border-radius: 14px;
  padding: 16px;

  &:hover {
    border-color: rgba(34, 197, 94, 0.4);
  }
}

// Server Promo Enhancement
:deep(.server-promo),
:deep([class*="server-promo"]) {
  background: linear-gradient(135deg, rgba(168, 85, 247, 0.08) 0%, rgba(168, 85, 247, 0.02) 100%);
  border: 1px solid rgba(168, 85, 247, 0.2);
  border-radius: 14px;
  padding: 16px;

  &:hover {
    border-color: rgba(168, 85, 247, 0.4);
  }
}

// ==========================================
// PAGE LAYOUT ENHANCEMENT
// ==========================================

// Main page wrapper
:deep(.new-page.sidebar) {
  max-width: 1400px;
  margin: 0 auto;
  padding: 24px 40px 60px;

  @media (max-width: 1200px) {
    padding: 20px 24px 48px;
  }

  @media (max-width: 768px) {
    padding: 16px 16px 40px;
  }
}

// Sidebar Enhancement
:deep(.normal-page__info) {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

// Content Area
:deep(.normal-page__content) {
  min-width: 0;
}

// Horizontal Rule Enhancement
:deep(hr) {
  border: none;
  height: 1px;
  background: var(--color-divider);
  margin: 12px 0;
}

// Crown Icon (Owner Badge)
:deep(.text-brand-orange) {
  color: var(--flame, #f16436) !important;
}

// ==========================================
// MESSAGE BANNERS
// ==========================================

:deep(.message-banner),
:deep([class*="message-banner"]) {
  border-radius: 14px;
  padding: 16px 20px;
  border: 1px solid;

  &.warning,
  &[class*="warning"] {
    background: rgba(249, 115, 22, 0.08);
    border-color: rgba(249, 115, 22, 0.3);
    color: #f97316;
  }

  &.error,
  &[class*="error"] {
    background: rgba(239, 68, 68, 0.08);
    border-color: rgba(239, 68, 68, 0.3);
    color: #ef4444;
  }

  &.info,
  &[class*="info"] {
    background: rgba(59, 130, 246, 0.08);
    border-color: rgba(59, 130, 246, 0.3);
    color: #3b82f6;
  }

  &.success,
  &[class*="success"] {
    background: rgba(34, 197, 94, 0.08);
    border-color: rgba(34, 197, 94, 0.3);
    color: #22c55e;
  }
}

// ==========================================
// OVERFLOW MENU ENHANCEMENT
// ==========================================

:deep(.overflow-menu),
:deep([class*="overflow-menu"]) {
  background: var(--bg-card, var(--color-raised-bg));
  border: 1px solid var(--color-divider);
  border-radius: 14px;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.15);
  overflow: hidden;

  .menu-item,
  [class*="menu-item"] {
    padding: 12px 16px;
    font-size: 0.875rem;
    transition: all 0.15s ease;

    &:hover {
      background: var(--accent-muted, rgba(241, 100, 54, 0.08));
    }

    svg {
      width: 18px;
      height: 18px;
      margin-right: 10px;
    }
  }
}

// ==========================================
// WIKI CREATE MODAL
// ==========================================

:deep(.wiki-create-modal),
:deep([class*="wiki-create"]) {
  input,
  textarea {
    background: var(--color-bg);
    border: 1px solid var(--color-divider);
    border-radius: 12px;
    padding: 12px 16px;
    font-size: 0.9rem;
    transition: all 0.2s ease;

    &:focus {
      border-color: var(--flame, #f16436);
      box-shadow: 0 0 0 3px rgba(241, 100, 54, 0.15);
      outline: none;
    }
  }
}

// ==========================================
// REVOLUTIONARY LAYOUT - IMMERSIVE HERO
// ==========================================

// Revolution Layout Container
// The .new-page.sidebar grid template is:
//   "header header" auto
//   "content sidebar" auto
//   "content dummy" 1fr
//   / 1fr 18.75rem
// We use the header area for the hero section
.revolution-layout {
  // Hero section takes the header grid area and breaks out to full viewport width
  .hero-section {
    grid-area: header;
    // Break out of container to full viewport width (FTB style)
    width: 100vw;
    position: relative;
    left: 50%;
    right: 50%;
    margin-left: -50vw;
    margin-right: -50vw;
  }

  // Keep sidebar and content in their original positions
  .normal-page__sidebar {
    grid-area: sidebar;
  }

  .normal-page__content {
    grid-area: content;
  }
}

// ==========================================
// HERO SECTION STYLES
// ==========================================

.hero-section {
  position: relative;
  width: 100%;
  min-height: 200px;
  margin-bottom: 24px;
}

// Hero Background - Clean, minimal design
// No blurred gallery image, just pure theme background
.hero-background {
  position: absolute;
  inset: 0;
  z-index: 0;
  background: var(--color-bg);

  // Hide the background image entirely for clean look
  .hero-bg-image {
    display: none;
  }

  // No gradient overlay needed
  .hero-gradient-overlay {
    display: none;
  }
}

// No gallery fallback
.hero-section:not(:has(.hero-bg-image)) {
  .hero-background {
    background: linear-gradient(
      135deg,
      var(--accent-muted, rgba(241, 100, 54, 0.15)) 0%,
      transparent 50%,
      var(--accent-muted, rgba(241, 100, 54, 0.08)) 100%
    );

    .hero-gradient-overlay {
      background: linear-gradient(180deg, transparent 0%, var(--color-bg, #0f0f0f) 100%);
    }
  }
}

// Hero Content Container
.hero-content {
  position: relative;
  z-index: 1;
  max-width: 1400px;
  margin: 0 auto;
  padding: 28px 40px 20px;
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 24px;

  @media (max-width: 1200px) {
    padding: 24px 24px 16px;
  }

  @media (max-width: 768px) {
    flex-direction: column;
    padding: 20px 16px 16px;
    gap: 20px;
  }
}

// Hero Main (Icon + Info)
.hero-main {
  display: flex;
  align-items: flex-start;
  gap: 24px;
  flex: 1;
  min-width: 0;

  @media (max-width: 640px) {
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 16px;
  }
}

// Hero Icon Wrapper
.hero-icon-wrapper {
  position: relative;
  flex-shrink: 0;

  :deep(.hero-icon) {
    border-radius: 20px !important;
    box-shadow:
      0 8px 32px rgba(0, 0, 0, 0.3),
      0 0 0 3px rgba(255, 255, 255, 0.1);
    transition:
      transform 0.3s var(--ease-out, ease),
      box-shadow 0.3s var(--ease-out, ease);

    &:hover {
      transform: scale(1.03);
      box-shadow:
        0 12px 40px rgba(241, 100, 54, 0.3),
        0 0 0 3px rgba(241, 100, 54, 0.3);
    }
  }
}

// Hero Info
.hero-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0;
  flex: 1;
}

.hero-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.hero-title {
  font-size: clamp(1.75rem, 4vw, 2.5rem);
  font-weight: 800;
  letter-spacing: -0.02em;
  color: var(--color-text-dark, #1a1a1a);
  margin: 0;
  line-height: 1.2;
}

.hero-status-badge {
  flex-shrink: 0;
}

.hero-description {
  font-size: 1rem;
  color: var(--color-secondary, #666);
  line-height: 1.6;
  margin: 0;
  max-width: 600px;

  @media (max-width: 640px) {
    font-size: 0.9rem;
  }
}

// Hero Tags
.hero-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  margin-top: 4px;

  @media (max-width: 640px) {
    justify-content: center;
  }
}

.hero-tag {
  display: inline-flex;
  align-items: center;
  padding: 6px 14px;
  background: var(--color-button-bg, #f0f0f0);
  border: 1px solid var(--color-divider, #e0e0e0);
  border-radius: 20px;
  font-size: 0.8rem;
  font-weight: 600;
  color: var(--color-text, #333);
  transition: all 0.2s ease;

  &:hover {
    background: var(--flame, #f16436);
    border-color: var(--flame, #f16436);
    color: #fff;
    transform: translateY(-1px);
  }

  &--more {
    background: rgba(241, 100, 54, 0.15);
    border-color: rgba(241, 100, 54, 0.3);
    color: var(--flame, #f16436);
  }
}

// Hero Actions
.hero-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;

  @media (max-width: 768px) {
    width: 100%;
    justify-content: center;
  }

  @media (max-width: 640px) {
    flex-wrap: wrap;
  }

  :deep(.hero-download-btn) {
    button {
      padding: 14px 28px !important;
      font-size: 1rem !important;
      font-weight: 700 !important;
      border-radius: 14px !important;
      background: linear-gradient(135deg, var(--flame, #f16436) 0%, #ff8a5c 100%) !important;
      box-shadow:
        0 8px 24px rgba(241, 100, 54, 0.4),
        0 0 0 1px rgba(255, 255, 255, 0.1) inset !important;

      &:hover {
        transform: translateY(-3px) !important;
        box-shadow:
          0 12px 32px rgba(241, 100, 54, 0.5),
          0 0 0 1px rgba(255, 255, 255, 0.15) inset !important;
      }
    }
  }
}

// ==========================================
// HERO META (TAGS + STATS COMBINED)
// ==========================================

.hero-meta {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 14px;
  margin-top: 10px;

  @media (max-width: 768px) {
    gap: 10px;
    margin-top: 8px;
  }
}

.hero-stats-inline {
  display: flex;
  align-items: center;
  gap: 16px;

  @media (max-width: 768px) {
    gap: 12px;
  }

  @media (max-width: 480px) {
    gap: 10px;
  }

  .stat-item {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-secondary, #666);
    transition: color 0.2s ease;

    &:hover {
      color: var(--color-text, #333);

      .stat-icon {
        color: var(--flame, #f16436);
      }
    }

    .stat-icon {
      width: 14px;
      height: 14px;
      opacity: 0.6;
      transition: all 0.2s ease;
    }

    @media (max-width: 480px) {
      font-size: 0.8rem;
      gap: 4px;

      .stat-icon {
        width: 12px;
        height: 12px;
      }
    }
  }
}

// ==========================================
// DARK THEME HERO ADAPTATIONS
// ==========================================
// Note: Using :global with high specificity to override scoped styles

// ==========================================
// ENHANCED SIDEBAR FOR REVOLUTION LAYOUT
// ==========================================

.revolution-layout {
  :deep(.normal-page__sidebar) {
    padding-top: 8px;

    .card.flex-card {
      background: var(--bg-card, var(--color-raised-bg));
      border: 1px solid var(--color-divider);
      border-radius: 20px;
      padding: 18px 20px;
      margin-bottom: 16px;
      transition: all 0.3s var(--ease-out, ease);

      &:hover {
        border-color: rgba(241, 100, 54, 0.3);
        box-shadow: 0 12px 32px rgba(0, 0, 0, 0.08);
      }

      // Enhanced Card Headers
      h2 {
        font-size: 1rem;
        font-weight: 700;
        margin-bottom: 8px;
        padding-bottom: 6px;
        display: flex;
        align-items: center;
        gap: 10px;

        &::before {
          content: "";
          width: 4px;
          height: 18px;
          background: linear-gradient(180deg, var(--flame, #f16436) 0%, #ff8a5c 100%);
          border-radius: 2px;
        }
      }

      // Enhanced Tag List - Clean, minimal background
      .tag-list {
        display: flex;
        flex-wrap: wrap;
        gap: 6px;

        .tag-list__item {
          padding: 4px 10px;
          border-radius: 8px;
          font-size: 0.8rem;
          font-weight: 500;
          background: transparent;
          border: 1px solid var(--color-divider);
          color: var(--color-text);
          transition: all 0.2s ease;

          svg {
            width: 14px;
            height: 14px;
            margin-right: 6px;
            color: var(--_color, var(--color-secondary));
          }

          // Platform tags with color - keep colored background
          &[style*="--_color"] {
            background: color-mix(in srgb, var(--_color) 12%, transparent);
            border-color: color-mix(in srgb, var(--_color) 30%, transparent);
            color: var(--_color);

            &:hover {
              background: color-mix(in srgb, var(--_color) 20%, transparent);
              border-color: var(--_color);
            }
          }

          &:hover {
            border-color: var(--flame, #f16436);
            color: var(--flame, #f16436);
          }
        }
      }

      // Enhanced Details List - Clean, no background
      .details-list {
        gap: 2px;

        .details-list__item {
          padding: 6px 4px;
          border-radius: 8px;
          background: transparent;
          border: none;

          &:hover {
            color: var(--flame, #f16436);

            svg {
              color: var(--flame, #f16436);
            }
          }
        }

        // Keep background only for member cards
        .details-list__item--type-large {
          padding: 10px 12px;
          background: var(--color-bg);
          border-radius: 10px;

          &:hover {
            background: var(--accent-muted, rgba(241, 100, 54, 0.08));
          }
        }
      }

      // Enhanced Links List - Clean, no background
      .links-list {
        display: flex;
        flex-direction: column;
        gap: 2px;

        a {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 6px 4px;
          background: transparent;
          border-radius: 8px;
          color: var(--color-text);
          text-decoration: none;
          transition: all 0.2s ease;

          svg {
            width: 18px;
            height: 18px;
            color: var(--flame, #f16436);
          }

          .external-icon {
            margin-left: auto;
            opacity: 0.4;
            width: 14px;
            height: 14px;
            color: var(--color-secondary);
          }

          &:hover {
            color: var(--flame, #f16436);

            .external-icon {
              opacity: 0.8;
              color: var(--flame, #f16436);
            }
          }
        }
      }
    }
  }

  // NavTabs component handles its own underline indicator
  :deep(.nav-tabs-underline) {
    margin-bottom: 20px;
  }
}
</style>

<style lang="scss">
// Non-scoped styles to override parent layout padding for immersive hero
// This cannot be in scoped styles because it needs to affect the parent main element
body main:has(.revolution-layout) {
  padding-top: 0 !important;
  margin-top: 0 !important;
}

// Override .new-page container padding for immersive hero
.new-page.revolution-layout {
  padding-top: 0 !important;
}

// ==========================================
// DARK THEME HERO ADAPTATIONS (Non-scoped)
// ==========================================
.dark-mode,
.oled-mode {
  .hero-section {
    .hero-title {
      color: #fff !important;
    }

    .hero-description {
      color: rgba(255, 255, 255, 0.75) !important;
    }

    .hero-tag {
      background: rgba(255, 255, 255, 0.1) !important;
      border-color: rgba(255, 255, 255, 0.2) !important;
      color: rgba(255, 255, 255, 0.9) !important;

      &:hover {
        background: var(--flame, #f16436) !important;
        border-color: var(--flame, #f16436) !important;
        color: #fff !important;
      }
    }

    .hero-stats-inline .stat-item {
      color: rgba(255, 255, 255, 0.65) !important;

      &:hover {
        color: rgba(255, 255, 255, 0.9) !important;
      }
    }
  }
}
</style>
