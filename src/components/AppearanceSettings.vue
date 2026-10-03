<script setup lang="ts">
import { useAppearanceStore } from "../stores/appearance";
import { styles, type Appearance } from "../lib/appearance";
const appearance = useAppearanceStore();
function change<K extends keyof Appearance>(key: K, event: Event) {
  appearance.set(
    key,
    (event.target as HTMLSelectElement).value as Appearance[K],
  );
}
</script>
<template>
  <details class="appearance-settings">
    <summary>外觀設定</summary>
    <div class="settings-fields">
      <label
        >介面風格<select
          aria-label="介面風格"
          :value="appearance.settings.style"
          @change="change('style', $event)"
        >
          <option
            v-for="style in styles"
            :key="style.value"
            :value="style.value"
          >
            {{ style.label }}
          </option>
        </select></label
      >
      <p>
        {{
          styles.find((s) => s.value === appearance.settings.style)?.description
        }}
      </p>
      <label
        >明暗主題<select
          aria-label="明暗主題"
          :value="appearance.settings.theme"
          @change="change('theme', $event)"
        >
          <option value="system">跟隨系統</option>
          <option value="light">明亮</option>
          <option value="dark">深色</option>
        </select></label
      >
      <label
        >書庫檢視<select
          aria-label="書庫檢視"
          :value="appearance.settings.view"
          @change="change('view', $event)"
        >
          <option value="grid">封面網格</option>
          <option value="detail">詳細列表</option>
          <option value="compact">緊湊列表</option>
        </select></label
      >
      <label
        >顯示密度<select
          aria-label="顯示密度"
          :value="appearance.settings.density"
          @change="change('density', $event)"
        >
          <option value="comfortable">舒適</option>
          <option value="compact">緊湊</option>
        </select></label
      >
      <label
        >網格封面尺寸<select
          aria-label="網格封面尺寸"
          :value="appearance.settings.coverSize"
          @change="change('coverSize', $event)"
        >
          <option value="small">小</option>
          <option value="medium">中</option>
          <option value="large">大</option>
        </select></label
      >
      <label><span><input type="checkbox" :checked="appearance.settings.readerPinned" @change="appearance.set('readerPinned', ($event.target as HTMLInputElement).checked)" /> 固定顯示閱讀工具列</span></label>
      <p v-if="appearance.saveFailed" role="status">
        設定暫時無法保存，本次仍可使用；重新啟動可能恢復預設。
      </p>
    </div>
  </details>
</template>
<style scoped>
summary {
  cursor: pointer;
  font-weight: 600;
  padding: 12px 0;
}
.settings-fields {
  display: grid;
  gap: 12px;
  padding-bottom: 16px;
}
label {
  display: grid;
  gap: 6px;
  font-size: 12px;
}
select {
  width: 100%;
}
p {
  font-size: 11px;
  line-height: 1.6;
  color: var(--text-dim);
}
</style>
