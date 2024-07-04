import { useEffect, useState } from 'react';
import reactLogo from './assets/react.svg';
import { invoke } from '@tauri-apps/api/tauri';
import './App.css';
import { UnlistenFn, listen } from '@tauri-apps/api/event';

type CriticaclErrorPayload = {
  event_name: string;
  detail: string | IpcError;
};

type ErrorPayload = CriticaclErrorPayload;

type VCSelectPayload = {
  in_vc: boolean;
};

type VCInfoPayload = {
  name: string;
  users: never; // TODO
};

type VCMuteUpdatePayload = {
  mute: boolean;
  deaf: boolean;
};

type IpcErrorType = 'CreateClient' | 'Connect' | 'Authorize' | 'Subscribe' | 'EventReceive' | 'EventEncode';

type IpcError = {
  error_type: IpcErrorType;
  message: string;
  payload?: unknown;
};

function App() {
  const [greetMsg, setGreetMsg] = useState('');
  const [name, setName] = useState('');
  const [retryCount, setRetryCount] = useState(0);
  const [inVC, setInVC] = useState(false);
  const [isMute, setIsMute] = useState(false);
  const [isDeafen, setIsDeafen] = useState(false);

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    setGreetMsg(await invoke('greet', { name }));
  }

  useEffect(() => {
    const unlistenFuncs: UnlistenFn[] = [];
    const initIPC = async () => {
      console.log('Started connecting to ipc...');
      invoke('connect_ipc')
        .then(() => {
          console.log('Connected to ipc.');
        })
        .catch((e: IpcError) => {
          if (e.error_type === 'CreateClient') {
            // kill the process cuz its un-recoverable
          } else if (e.error_type === 'Connect') {
            // discord client might be not running
            // schedule the `connect_ipc` command in 10 secs or smth
          } else if (e.error_type === 'Authorize') {
            if (retryCount < 5) {
              // retry in 5 secs
              // setTimeout...
              setRetryCount(retryCount + 1);
            }
          } else if (e.error_type === 'Subscribe') {
          }
        });

      const unlistenCriticalError = await listen<CriticaclErrorPayload>('critical_error', (e) => {
        console.error(`Critical Error: ${e.payload.event_name}\n  ${e.payload.detail}`);
        // Show dialog and kill the process
      });
      unlistenFuncs.push(unlistenCriticalError);

      const unlistenError = await listen<ErrorPayload>('error', (e) => {
        console.error(`Error: ${e.payload.event_name}\n  ${e.payload.detail}`);
        const error = e.payload.detail;
      });
      unlistenFuncs.push(unlistenError);

      const unlistenVCSelect = await listen<VCSelectPayload>('vc_select', (e) => {
        setInVC(e.payload.in_vc);

        console.log(e.payload);
      });
      unlistenFuncs.push(unlistenVCSelect);

      const unlistenVCSelectInfo = await listen<VCSelectPayload>('vc_info', (e) => {
        // TODO
        console.log(e.payload);
      });
      unlistenFuncs.push(unlistenVCSelectInfo);

      const unlistenVCUpdate = await listen<VCMuteUpdatePayload>('vc_mute_update', (e) => {
        setIsMute(e.payload.mute);
        setIsDeafen(e.payload.deaf);
        console.log(e.payload);
      });
      unlistenFuncs.push(unlistenVCUpdate);

      const unlistenVcInfo = await listen<VCInfoPayload>('vc_info', (e) => {
        // set info
        console.log(e.payload);
      });
      unlistenFuncs.push(unlistenVcInfo);
    };

    initIPC();

    return () => {
      unlistenFuncs.forEach((f) => {
        f();
      });
    };
  }, []);

  return (
    <div className="container">
      <h1>Welcome to Tauri!</h1>

      <div className="row">
        <a href="https://vitejs.dev" target="_blank">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://reactjs.org" target="_blank">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>

      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input id="greet-input" onChange={(e) => setName(e.currentTarget.value)} placeholder="Enter a name..." />
        <button type="submit">Greet</button>
      </form>

      <p>{greetMsg}</p>
    </div>
  );
}

export default App;
