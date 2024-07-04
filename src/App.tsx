import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { IpcError, RustError } from './utils/error';
import { Box } from '@mui/material';

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

function App() {
  const [retryCount, setRetryCount] = useState(0);
  const [inVC, setInVC] = useState(false);
  const [isMute, setIsMute] = useState(false);
  const [isDeafen, setIsDeafen] = useState(false);

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

      const unlistenCriticalError = await listen<RustError>('critical_error', (e) => {
        console.error(`Critical Error: ${e.payload.error_type}\n  ${e.payload.message}`);
        // Show dialog and kill the process
      });
      unlistenFuncs.push(unlistenCriticalError);

      const unlistenError = await listen<RustError>('error', (e) => {
        console.error(`Error: ${e.payload.error_type}\n  ${e.payload.message}`);
      });
      unlistenFuncs.push(unlistenError);

      const unlistenVCSelect = await listen<VCSelectPayload>('vc_select', (e) => {
        setInVC(e.payload.in_vc);

        console.log(e.payload);
      });
      unlistenFuncs.push(unlistenVCSelect);

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
    <Box height={`${window.innerHeight}px`}>
      <Box height={'20%'}>VC MUTE INFO</Box>
      <Box height={'80%'}>VC MEMBER INFO</Box>
    </Box>
  );
}

export default App;
