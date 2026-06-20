/**
 * 前端应用入口
 *
 * 初始化 React 根渲染，注入 StrictMode 和 ThemeProvider。
 */
import '@douyinfe/semi-ui/react19-adapter';
import './styles.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App.tsx';
import { ThemeProvider } from './hooks/useTheme.ts';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <ThemeProvider>
      <App/>
    </ThemeProvider>
  </StrictMode>,
);