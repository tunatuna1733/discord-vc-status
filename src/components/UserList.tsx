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
          <Avatar alt={userData.id} src={`https://cdn.discordapp.com/avatars/${userData.id}/${userData.avatar}.png`} />
        </ListItemAvatar>
        <ListItemText>{userData.nick}</ListItemText>
        <Grid container textAlign={'right'}>
          <Grid item xs={6}>
            {userData.mute ? <MicOffIcon /> : <MicIcon />}
          </Grid>
          <Grid item xs={6}>
            {userData.deaf ? <HeadsetOffIcon /> : <HeadsetIcon />}
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
