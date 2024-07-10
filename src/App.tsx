import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { UnlistenFn, listen } from '@tauri-apps/api/event';
import { IpcError, RustError } from './utils/error';
import { UserData } from './types/user';
import { VCSelectPayload, VCMuteUpdatePayload, VCInfoPayload, VCUserPayload, VCSpeakPayload } from './types/event';
import { formatUserData } from './utils/vc';
import VCSettings from './components/VCSettings';
import { Box, Grid } from '@mui/material';
import UserList from './components/UserList';
import SetActivity from './components/SetActivity';

function App() {
  const [retryCount, setRetryCount] = useState(0);
  const [inVC, setInVC] = useState(false);
  const [isMute, setIsMute] = useState(false);
  const [isDeafen, setIsDeafen] = useState(false);
  const [isSpeaking, setIsSpeaking] = useState(false);
  const [userList, setUserList] = useState<UserData[]>([]);
  const [vcName, setVCName] = useState('');

  const userListRef = useRef<UserData[]>();
  userListRef.current = userList;

  const joinUser = (userData: VCUserPayload) => {
    if (userListRef.current) {
      const mute = userData.data.mute || userData.data.self_mute || userData.data.deaf || userData.data.self_deaf;
      const deaf = userData.data.deaf || userData.data.self_deaf;
      const data: UserData = {
        id: userData.data.id,
        username: userData.data.username,
        avatar: userData.data.avatar,
        nick: userData.data.nick,
        mute,
        deaf,
        speaking: false,
      };
      setUserList([...userListRef.current, data]);
    }
  };

  const updateUser = (userData: VCUserPayload) => {
    if (userListRef.current) {
      const currentData = userListRef.current.find((u) => u.id === userData.data.id);
      const mute = userData.data.mute || userData.data.self_mute || userData.data.deaf || userData.data.self_deaf;
      const deaf = userData.data.deaf || userData.data.self_deaf;
      const newData: UserData = {
        id: userData.data.id,
        username: userData.data.username,
        avatar: userData.data.avatar,
        nick: userData.data.nick,
        mute,
        deaf,
        speaking: false,
      };
      if (!currentData) {
        setUserList([...userListRef.current, newData]);
      } else {
        newData.speaking = currentData.speaking;
        setUserList([...userListRef.current.filter((u) => u.id !== userData.data.id), newData]);
      }
    }
  };

  const leaveUser = (userData: VCUserPayload) => {
    if (userListRef.current) {
      setUserList(userListRef.current.filter((u) => u.id !== userData.data.id));
    }
  };

  const getVCInfo = () => {
    invoke<VCInfoPayload & VCSelectPayload>('get_vc_info')
      .then((r) => {
        console.log(r);
        if (r.in_vc) {
          setVCName(r.name);
          const currentUserList = r.users.map(formatUserData);
          console.log(currentUserList);
          setUserList(currentUserList);
        }
      })
      .catch((e: IpcError) => {
        console.error(e);
      });
  };

  useEffect(() => {
    const unlistenFuncs: UnlistenFn[] = [];
    const initIPC = async () => {
      console.log('Started connecting to ipc...');
      invoke('connect_ipc', { reauth: true })
        .then(() => {
          console.log('Connected to ipc.');
        })
        .catch((e: IpcError) => {
          if (e.error_type === 'CreateClient') {
            // kill the process cuz its un-recoverable
          } else if (e.error_type === 'Connect') {
            // discord client might be not running
            // schedule the `connect_ipc` command in 10 secs or smth
          } else if (e.error_type === 'ReAuth') {
            console.error('Failed to reauth. Trying to do normal auth...');
            console.error(e.message);
            invoke('connect_ipc', { reauth: false });
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
        } else {
          getVCInfo();
        }
      });
      unlistenFuncs.push(unlistenVCSelect);

      const unlistenVCUpdate = await listen<VCMuteUpdatePayload>('vc_mute_update', (e) => {
        const mute = e.payload.deaf ? true : e.payload.mute;
        setIsMute(mute);
        setIsDeafen(e.payload.deaf);
      });
      unlistenFuncs.push(unlistenVCUpdate);

      const unlistenVcInfo = await listen<VCInfoPayload>('vc_info', (e) => {
        console.log(e.payload.users);
        /*
        setVCName(e.payload.name);
        setUserList(
          e.payload.users.map((u) => {
            const mute = u.voice_state.mute || u.voice_state.self_mute || u.voice_state.deaf || u.voice_state.self_deaf;
            const deaf = u.voice_state.deaf || u.voice_state.self_deaf;
            const userData: UserData = {
              id: u.user.id,
              username: u.user.username,
              avatar: u.user.avatar,
              nick: u.nick,
              mute,
              deaf,
              speaking: false,
            };
            return userData;
          })
        );
        */
      });
      unlistenFuncs.push(unlistenVcInfo);

      const unlistenVcUser = await listen<VCUserPayload>('vc_user', (e) => {
        const rawData = e.payload;
        console.log('vc_user event');
        console.log(rawData);
        if (rawData.event === 'JOIN') {
          joinUser(rawData);
        } else if (rawData.event === 'UPDATE') {
          updateUser(rawData);
        } else if (rawData.event === 'LEAVE') {
          leaveUser(rawData);
        }
      });
      unlistenFuncs.push(unlistenVcUser);

      const unlistenVcSpeak = await listen<VCSpeakPayload>('vc_speak', (e) => {
        console.log('vc_speak event');
        console.log(e.payload);
        if (e.payload.is_me) {
          setIsSpeaking(e.payload.speaking);
        } else {
          if (userListRef.current) {
            const userData = userListRef.current.find((u) => u.id === e.payload.user_id);
            if (userData) {
              const newData = structuredClone(userData);
              newData.speaking = e.payload.speaking;
              setUserList([...userListRef.current.filter((u) => u.id !== e.payload.user_id), newData]);
            }
          }
        }
      });
      unlistenFuncs.push(unlistenVcSpeak);
    };

    initIPC();

    return () => {
      console.log('useEffect returned');
      unlistenFuncs.forEach((f) => {
        f();
      });
      invoke('disconnect_ipc');
    };
  }, []);

  useEffect(() => {
    console.log(userList);
  }, [userList]);

  return (
    <Box height={`${window.innerHeight}px`}>
      <Box height={'10%'}>
        <VCSettings inVC={inVC} isMute={isMute} isDeafen={isDeafen} isSpeaking={isSpeaking} vcName={vcName} />
      </Box>
      <Box>
        <Grid container>
          <Grid item xs={7}>
            <UserList userList={userList} />
          </Grid>
          <Grid item xs={5}>
            <SetActivity />
          </Grid>
        </Grid>
      </Box>
    </Box>
  );
}

export default App;
