import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { IpcError, RustError } from './utils/error';
import { Box, Grid, IconButton } from '@mui/material';
import SettingsPhoneIcon from '@mui/icons-material/SettingsPhone';
import MicIcon from '@mui/icons-material/Mic';
import MicOffIcon from '@mui/icons-material/MicOff';
import HeadsetIcon from '@mui/icons-material/Headset';
import HeadsetOffIcon from '@mui/icons-material/HeadsetOff';
import CallEndIcon from '@mui/icons-material/CallEnd';
import { UserData } from './types/user';
import { VCSelectPayload, VCMuteUpdatePayload, VCInfoPayload, VCUserPayload, VCSpeakPayload } from './types/event';
import { formatUserData } from './utils/vc';

function App() {
  const [retryCount, setRetryCount] = useState(0);
  const [inVC, setInVC] = useState(false);
  const [isMute, setIsMute] = useState(false);
  const [isDeafen, setIsDeafen] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [userList, setUserList] = useState<UserData[]>([]);
  const [vcName, setVCName] = useState('');

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
              setRetryCount((prev) => prev + 1);
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
        if (!e.payload.in_vc) {
          setVCName('');
          setUserList([]);
          setIsSpeaking(false);
        }
      });
      unlistenFuncs.push(unlistenVCSelect);

      const unlistenVCUpdate = await listen<VCMuteUpdatePayload>('vc_mute_update', (e) => {
        setIsMute(e.payload.mute);
        setIsDeafen(e.payload.deaf);
      });
      unlistenFuncs.push(unlistenVCUpdate);

      const unlistenVcInfo = await listen<VCInfoPayload>('vc_info', (e) => {
        setVCName(e.payload.name);
        const currentUserList: UserData[] = [];
        e.payload.users.forEach((u) => {
          const userData = formatUserData(u);
          currentUserList.push(userData);
        });
        setUserList(currentUserList);
      });
      unlistenFuncs.push(unlistenVcInfo);

      const unlistenVcUser = await listen<VCUserPayload>('vc_user', (e) => {
        const rawData = e.payload;
        if (rawData.event === 'JOIN') {
          const mute = rawData.data.mute || rawData.data.self_mute || rawData.data.deaf || rawData.data.self_deaf;
          const deaf = rawData.data.deaf || rawData.data.self_deaf;
          const data: UserData = {
            id: rawData.data.id,
            username: rawData.data.username,
            avatar: rawData.data.avatar,
            nick: rawData.data.nick,
            mute,
            deaf,
            speaking: false,
          };
          setUserList([...userList, data]);
        } else if (rawData.event === 'UPDATE') {
          const currentData = userList.find((u) => u.id === rawData.data.id);
          const mute = rawData.data.mute || rawData.data.self_mute || rawData.data.deaf || rawData.data.self_deaf;
          const deaf = rawData.data.deaf || rawData.data.self_deaf;
          const newData: UserData = {
            id: rawData.data.id,
            username: rawData.data.username,
            avatar: rawData.data.avatar,
            nick: rawData.data.nick,
            mute,
            deaf,
            speaking: false,
          };
          if (!currentData) {
            setUserList([...userList, newData]);
          } else {
            newData.speaking = currentData.speaking;
            setUserList([...userList.filter((u) => u.id !== rawData.data.id), newData]);
          }
        } else if (rawData.event === 'LEAVE') {
          setUserList(userList.filter((u) => u.id !== rawData.data.id));
        }
      });
      unlistenFuncs.push(unlistenVcUser);

      const unlistenVcSpeak = await listen<VCSpeakPayload>('vc_speak', (e) => {
        if (e.payload.is_me) {
          setIsSpeaking(e.payload.speaking);
        } else {
          const userData = userList.find((u) => u.id === e.payload.user_id);
          if (userData) {
            const newData = structuredClone(userData);
            newData.speaking = e.payload.speaking;
            setUserList([...userList.filter((u) => u.id !== e.payload.user_id), newData]);
          }
        }
      });
      unlistenFuncs.push(unlistenVcSpeak);
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
      <Box height={'10%'}>
        <Grid container>
          <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
            <IconButton aria-label="chatting">
              <SettingsPhoneIcon />
            </IconButton>
          </Grid>
          <Grid item xs={8}>
            {vcName}
          </Grid>
          <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
            <IconButton aria-label="mute">
              {inVC ? (
                isMute ? (
                  <MicOffIcon color="error" />
                ) : isSpeaking ? (
                  <MicIcon color="success" />
                ) : (
                  <MicIcon />
                )
              ) : isMute ? (
                <MicOffIcon color="disabled" />
              ) : (
                <MicIcon color="disabled" />
              )}
            </IconButton>
          </Grid>
          <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
            <IconButton aria-label="deafen">
              {inVC ? (
                isDeafen ? (
                  <HeadsetOffIcon color="error" />
                ) : (
                  <HeadsetIcon />
                )
              ) : isDeafen ? (
                <HeadsetOffIcon color="disabled" />
              ) : (
                <HeadsetIcon color="disabled" />
              )}
            </IconButton>
          </Grid>
          <Grid item xs={1} display={'flex'} alignItems={'center'} justifyContent={'center'}>
            <IconButton aria-label="disconnect">
              <CallEndIcon />
            </IconButton>
          </Grid>
        </Grid>
      </Box>
      <Box>{JSON.stringify(userList)}</Box>
    </Box>
  );
}

export default App;
