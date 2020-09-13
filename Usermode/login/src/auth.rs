
pub enum Error
{
	InvalidAuthentication,
	Disabled,
}

pub struct UserInfo
{
}

pub fn try_login(username: &str, password: &str) -> Result<UserInfo, Error>
{
	// TODO: Use a proper auth infrastructure, something PAM-esque
	if username == "root" && password == "password"
	{
		Ok(UserInfo {})
	}
	else if username == "guest"
	{
		Err(Error::Disabled)
	}
	else
	{
		Err(Error::InvalidAuthentication)
	}
}


impl UserInfo
{
	pub fn get_shell(&self) -> &str
	{
		"/sysroot/bin/shell"
	}
}

