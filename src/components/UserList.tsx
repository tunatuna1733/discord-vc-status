import { Avatar, Grid, List, ListItem, ListItemAvatar, ListItemButton, ListItemText } from '@mui/material';
import MicIcon from '@mui/icons-material/Mic';
import MicOffIcon from '@mui/icons-material/MicOff';
import HeadsetIcon from '@mui/icons-material/Headset';
import HeadsetOffIcon from '@mui/icons-material/HeadsetOff';
import { UserData } from '../types/user';
import { useEffect } from 'react';

const UserItem = ({ userData }: { userData: UserData }) => {
  return (
    <ListItem>
      <ListItemButton>
        <ListItemAvatar>
          <Avatar
            alt={userData.id}
            src={`https://cdn.discordapp.com/avatars/${userData.id}/${userData.avatar}.png`}
            style={userData.speaking ? { outline: '2px solid green' } : {}}
          />
        </ListItemAvatar>
        <Grid container>
          <Grid item xs={9}>
            <ListItemText>{userData.nick}</ListItemText>
          </Grid>
          <Grid item xs={3} display={'flex'} alignItems={'center'} justifyContent={'center'}>
            <Grid container>
              <Grid item xs={6} display={'flex'} alignItems={'center'} justifyContent={'center'}>
                {userData.mute ? <MicOffIcon /> : <MicIcon />}
              </Grid>
              <Grid item xs={6} display={'flex'} alignItems={'center'} justifyContent={'center'}>
                {userData.deaf ? <HeadsetOffIcon /> : <HeadsetIcon />}
              </Grid>
            </Grid>
          </Grid>
        </Grid>
      </ListItemButton>
    </ListItem>
  );
};

const UserList = ({ userList }: { userList: UserData[] }) => {
  useEffect(() => {
    console.log(userList);
  }, [userList]);
  return (
    <List>
      {userList.map((userData, i) => (
        <UserItem userData={userData} key={i} />
      ))}
    </List>
  );
};

export default UserList;
