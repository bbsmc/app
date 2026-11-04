<template>
  <div ref="root" class="date-picker">
    <button
      type="button"
      class="date-picker__trigger"
      :aria-label="label"
      aria-haspopup="dialog"
      :aria-expanded="open"
      @click="togglePanel"
    >
      <CalendarIcon class="date-picker__trigger-icon" aria-hidden="true" />
      <span class="date-picker__trigger-label">{{ triggerLabel }}</span>
      <DropdownIcon class="date-picker__trigger-chevron" aria-hidden="true" />
    </button>

    <div
      v-if="open"
      class="date-picker__panel"
      :class="{ 'date-picker__panel--right': align === 'right' }"
      role="dialog"
      :aria-label="label"
    >
      <div class="date-picker__head">
        <button type="button" class="date-picker__nav" aria-label="上一月" @click="shiftMonth(-1)">
          <ChevronLeftIcon aria-hidden="true" />
        </button>
        <span class="date-picker__month">{{ viewMonth.format("YYYY年M月") }}</span>
        <button
          type="button"
          class="date-picker__nav"
          aria-label="下一月"
          :disabled="!canGoNext"
          @click="shiftMonth(1)"
        >
          <ChevronRightIcon aria-hidden="true" />
        </button>
      </div>
      <div class="date-picker__weekdays">
        <span v-for="weekday in weekdays" :key="weekday">{{ weekday }}</span>
      </div>
      <div class="date-picker__grid">
        <button
          v-for="cell in cells"
          :key="cell.key"
          type="button"
          class="date-picker__day"
          :class="cell.classes"
          :title="cell.title"
          :disabled="cell.disabled"
          @click="selectDay(cell)"
        >
          {{ cell.label }}
        </button>
      </div>
      <div class="date-picker__foot">
        <button type="button" class="date-picker__quick" @click="selectToday">今天</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { CalendarIcon, ChevronLeftIcon, ChevronRightIcon, DropdownIcon } from "@modrinth/assets";
import dayjs from "dayjs";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";

const props = defineProps<{
  /** 选中日期（毫秒时间戳，当天任意时刻） */
  modelValue: number;
  /** 触发器与面板的无障碍名称 */
  label?: string;
  /** 面板对齐方向，靠右侧的选择器用 right 防小屏溢出 */
  align?: "left" | "right";
  /** 视觉提示上界：晚于该日期的格子弱化显示（仍可点击，由父级联动调整另一端） */
  maxDate?: number;
}>();

const emit = defineEmits(["update:modelValue", "change"]);

const today = dayjs().startOf("day");
const weekdays = ["一", "二", "三", "四", "五", "六", "日"];

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const viewMonth = ref(dayjs(props.modelValue).startOf("month"));

const triggerLabel = computed(() => dayjs(props.modelValue).format("YYYY-MM-DD"));
const canGoNext = computed(() => viewMonth.value.isBefore(today.startOf("month"), "month"));

interface DayCell {
  key: number;
  label: number;
  disabled: boolean;
  title?: string;
  classes: Record<string, boolean>;
}

const cells = computed<DayCell[]>(() => {
  const first = viewMonth.value.startOf("month");
  const offset = (first.day() + 6) % 7; // 周一为第一列
  const gridStart = first.subtract(offset, "day");
  const selectedDay = dayjs(props.modelValue).startOf("day").valueOf();
  const maxDay = props.maxDate ? dayjs(props.maxDate).startOf("day").valueOf() : null;

  return Array.from({ length: 42 }, (_, i) => {
    const d = gridStart.add(i, "day");
    const ts = d.valueOf();
    const disabled = d.isAfter(today);
    const classes: Record<string, boolean> = {
      "date-picker__day--outside": !d.isSame(viewMonth.value, "month"),
      "date-picker__day--today": ts === today.valueOf(),
      "date-picker__day--selected": ts === selectedDay,
      "date-picker__day--beyond": maxDay !== null && ts > maxDay && !disabled && ts !== selectedDay,
    };
    return {
      key: ts,
      label: d.date(),
      disabled,
      title: maxDay !== null && ts > maxDay ? "选择后另一端日期将同步调整" : undefined,
      classes,
    };
  });
});

const togglePanel = () => {
  if (open.value) {
    closePanel();
  } else {
    viewMonth.value = dayjs(props.modelValue).startOf("month");
    open.value = true;
  }
};

const closePanel = () => {
  open.value = false;
};

const shiftMonth = (delta: number) => {
  viewMonth.value = viewMonth.value.add(delta, "month");
};

const commit = (ts: number) => {
  emit("update:modelValue", ts);
  emit("change", ts);
  closePanel();
};

const selectDay = (cell: DayCell) => {
  if (!cell.disabled) {
    commit(cell.key);
  }
};

const selectToday = () => {
  commit(today.valueOf());
};

const onDocMouseDown = (event: MouseEvent) => {
  if (open.value && root.value && !root.value.contains(event.target as Node)) {
    closePanel();
  }
};

const onDocKeyDown = (event: KeyboardEvent) => {
  if (open.value && event.key === "Escape") {
    closePanel();
  }
};

onMounted(() => {
  document.addEventListener("mousedown", onDocMouseDown);
  document.addEventListener("keydown", onDocKeyDown);
});

onBeforeUnmount(() => {
  document.removeEventListener("mousedown", onDocMouseDown);
  document.removeEventListener("keydown", onDocKeyDown);
});
</script>

<style scoped lang="scss">
.date-picker {
  position: relative;
}

.date-picker__trigger {
  display: flex;
  align-items: center;
  gap: var(--gap-sm);
  height: 40px;
  padding: 0 var(--gap-md);
  background-color: var(--color-button-bg);
  color: var(--color-contrast);
  border: none;
  border-radius: var(--radius-md);
  box-shadow:
    var(--shadow-inset-sm),
    0 0 0 0 transparent;
  font-size: var(--font-size-nm);
  white-space: nowrap;
  cursor: pointer;
  transition: 0.05s;

  &:hover {
    filter: brightness(1.1);
  }

  &:focus {
    outline: 0;
    filter: brightness(1.25);
  }
}

.date-picker__trigger-icon,
.date-picker__trigger-chevron {
  color: var(--color-secondary);
}

.date-picker__trigger-label {
  font-variant-numeric: tabular-nums;
}

.date-picker__panel {
  position: absolute;
  top: calc(100% + var(--gap-xs));
  left: 0;
  z-index: 20;
  width: 17rem;
  max-width: calc(100vw - 2rem);
  display: flex;
  flex-direction: column;
  gap: var(--gap-sm);
  padding: var(--gap-md);
  background-color: var(--color-raised-bg);
  border: 1px solid var(--color-button-bg);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-floating);
  transform-origin: top left;
  animation: date-picker-in 0.12s ease-out;

  &--right {
    left: auto;
    right: 0;
    transform-origin: top right;
  }
}

@keyframes date-picker-in {
  from {
    opacity: 0;
    transform: translateY(-4px);
  }

  to {
    opacity: 1;
    transform: none;
  }
}

.date-picker__head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.date-picker__month {
  font-weight: var(--font-weight-bold);
  color: var(--color-contrast);
}

.date-picker__nav {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.75rem;
  height: 1.75rem;
  padding: 0;
  background: transparent;
  color: var(--color-secondary);
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;

  svg {
    width: 1rem;
    height: 1rem;
  }

  &:hover:not(:disabled) {
    background-color: var(--color-button-bg);
    color: var(--color-contrast);
  }

  &:disabled {
    opacity: 0.35;
    cursor: default;
  }
}

.date-picker__weekdays {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  text-align: center;
  color: var(--color-secondary);
  font-size: var(--font-size-xs);
}

.date-picker__grid {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 2px;
}

.date-picker__day {
  display: flex;
  align-items: center;
  justify-content: center;
  aspect-ratio: 1;
  padding: 0;
  background: transparent;
  color: var(--color-contrast);
  border: none;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  font-variant-numeric: tabular-nums;
  cursor: pointer;

  &:hover:not(:disabled) {
    background-color: var(--color-button-bg);
  }

  &:disabled {
    opacity: 0.35;
    cursor: default;
  }

  &--outside {
    color: var(--color-secondary);
    opacity: 0.55;
  }

  &--beyond {
    opacity: 0.45;
  }

  &--today {
    box-shadow: inset 0 0 0 1px var(--color-brand);
  }

  &--selected {
    background-color: var(--color-brand);
    color: var(--color-accent-contrast);
    font-weight: var(--font-weight-bold);
    box-shadow: none;
  }
}

.date-picker__foot {
  display: flex;
  justify-content: flex-end;
}

.date-picker__quick {
  padding: var(--gap-xs) var(--gap-md);
  background: transparent;
  color: var(--color-brand);
  border: none;
  border-radius: var(--radius-sm);
  font-size: var(--font-size-sm);
  cursor: pointer;

  &:hover {
    background-color: var(--color-button-bg);
  }
}
</style>
