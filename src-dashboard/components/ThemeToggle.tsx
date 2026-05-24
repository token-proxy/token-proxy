import type { ReactNode } from 'react';
import { Button, Dropdown } from '@douyinfe/semi-ui';
import { IconDesktop, IconMoon, IconSun, IconTick } from '@douyinfe/semi-icons';
import { useTheme, type ThemeMode } from '../hooks/useTheme';

interface ThemeToggleProps {
  size?: 'default' | 'small' | 'large';
}

const themeOptions: Array<{
  mode: ThemeMode;
  label: string;
  icon: ReactNode;
}> = [
  { mode: 'light', label: '明亮模式', icon: <IconSun /> },
  { mode: 'dark', label: '黑暗模式', icon: <IconMoon /> },
  { mode: 'system', label: '跟随系统', icon: <IconDesktop /> },
];

export default function ThemeToggle({ size = 'default' }: ThemeToggleProps): ReactNode {
  const { mode, setMode } = useTheme();
  const currentOption = themeOptions.find((option) => option.mode === mode) ?? themeOptions[2];

  return (
    <Dropdown
      clickToHide
      trigger="click"
      render={
        <Dropdown.Menu>
          {themeOptions.map((option) => (
            <Dropdown.Item
              key={option.mode}
              active={mode === option.mode}
              icon={option.icon}
              onClick={() => setMode(option.mode)}
            >
              <span style={{ display: 'inline-flex', alignItems: 'center', gap: 8 }}>
                {option.label}
                {mode === option.mode && <IconTick size="small" />}
              </span>
            </Dropdown.Item>
          ))}
        </Dropdown.Menu>
      }
    >
      <Button
        aria-label={`当前主题: ${currentOption.label}`}
        icon={currentOption.icon}
        size={size}
        theme="borderless"
        type="tertiary"
      >
        {currentOption.label}
      </Button>
    </Dropdown>
  );
}
