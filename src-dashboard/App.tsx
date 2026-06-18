import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom';
import AdminLayout from './layouts/AdminLayout.tsx';
import LoginPage from './pages/LoginPage.tsx';
import DashboardPage from './pages/DashboardPage.tsx';
import ProviderManagement from './pages/ProviderManagement.tsx';
import AccessPointManagement from './pages/AccessPointManagement.tsx';
import UserManagement from './pages/UserManagement.tsx';
import SessionLogPage from './pages/SessionLogPage.tsx';
import RequestLogPage from './pages/RequestLogPage.tsx';
import LogDetailPage from './pages/LogDetailPage.tsx';
import SettingsPage from './pages/SettingsPage.tsx';
import ProfilePage from './pages/ProfilePage.tsx';

function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage/>}/>
        <Route path="/" element={<AdminLayout/>}>
          <Route index element={<Navigate to="/dashboard" replace/>}/>
          <Route path="dashboard" element={<DashboardPage/>}/>
          <Route path="providers/*" element={<ProviderManagement/>}/>
          <Route path="access-points" element={<AccessPointManagement/>}/>
          <Route path="sessions" element={<SessionLogPage/>}/>
          <Route path="sessions/:sessionId" element={<SessionLogPage/>}/>
          <Route path="logs" element={<RequestLogPage/>}/>
          <Route path="logs/:id" element={<LogDetailPage/>}/>
          <Route path="users" element={<UserManagement/>}/>
          <Route path="settings" element={<SettingsPage/>}/>
          <Route path="settings/profile" element={<ProfilePage/>}/>
        </Route>
        <Route path="*" element={<Navigate to="/dashboard" replace/>}/>
      </Routes>
    </BrowserRouter>
  );
}

export default App;