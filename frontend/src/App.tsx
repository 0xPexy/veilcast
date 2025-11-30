import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Navigate, Route, BrowserRouter as Router, Routes } from 'react-router-dom';
import { WagmiProvider, createConfig, http } from 'wagmi';
import { injected } from 'wagmi/connectors';
import { mainnet, sepolia } from 'wagmi/chains';
import { Navbar } from './components/Navbar';
import { HomePage } from './pages/Home';
import { CreatePollPage } from './pages/CreatePoll';
import { LeaderboardPage } from './pages/Leaderboard';
import { ProfilePage } from './pages/Profile';
import { PollDetailPage } from './pages/PollDetail';

const queryClient = new QueryClient();
const chainId = Number(import.meta.env.VITE_CHAIN_ID ?? 11155111);
const rpcUrl = import.meta.env.VITE_RPC_URL ?? 'https://rpc.sepolia.org';
const chain = [mainnet, sepolia].find((c) => c.id === chainId) ?? sepolia;

const wagmiConfig = createConfig({
  chains: [chain],
  connectors: [injected()],
  transports: {
    [chain.id]: http(rpcUrl),
  },
});

export default function App() {
  return (
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <Router>
          <Navbar />
          <main className="mx-auto flex max-w-6xl flex-col gap-8 px-6 pb-16 pt-8">
            <Routes>
              <Route path="/" element={<HomePage />} />
              <Route path="/create" element={<CreatePollPage />} />
              <Route path="/leaderboard" element={<LeaderboardPage />} />
              <Route path="/profile" element={<ProfilePage />} />
              <Route path="/poll/:id" element={<PollDetailPage />} />
              <Route path="*" element={<Navigate to="/" replace />} />
            </Routes>
          </main>
        </Router>
      </QueryClientProvider>
    </WagmiProvider>
  );
}
