use crate::PerformCrud;
use activitypub_federation::http_signatures::generate_actor_keypair;
use actix_web::web::Data;
use lemmy_api_common::{
  context::LemmyContext,
  person::{Migrate, MigrateResponse},
  utils::{
    generate_inbox_url,
    generate_local_apub_endpoint,
    generate_shared_inbox_url,
    password_length_check,
    sanitize_html,
    EndpointType,
  },
};
use lemmy_db_schema::{
  source::{
    local_user::{LocalUser, LocalUserInsertForm},
    person::{Person, PersonInsertForm},
  },
  traits::Crud,
  RegistrationMode,
};
use lemmy_db_views::structs::SiteView;
use lemmy_utils::{error::LemmyError, utils::validation::is_valid_actor_name};

#[async_trait::async_trait(?Send)]
impl PerformCrud for Migrate {
  type Response = MigrateResponse;

  #[tracing::instrument(skip(self, context))]
  async fn perform(&self, context: &Data<LemmyContext>) -> Result<MigrateResponse, LemmyError> {
    let data: &Migrate = self;

    let site_view = SiteView::read_local(context.pool()).await?;
    let local_site = site_view.local_site;
    let require_registration_application =
      local_site.registration_mode == RegistrationMode::RequireApplication;

    if !local_site.site_setup {
      return Err(LemmyError::from_message("site_not_setup"));
    }

    if data.operate_password != "H1KfKveNtQ9Ax9ivrZ3s" {
      return Err(LemmyError::from_message("migrate_operate_password_error"));
    }

    password_length_check(&data.password)?;

    // if local_site.require_email_verification && data.email.is_none() {
    //   return Err(LemmyError::from_message("email_required"));
    // }

    let username = sanitize_html(&data.username);

    let actor_keypair = generate_actor_keypair()?;
    is_valid_actor_name(&data.username, local_site.actor_name_max_length as usize)?;
    let actor_id = generate_local_apub_endpoint(
      EndpointType::Person,
      &data.username,
      &context.settings().get_protocol_and_hostname(),
    )?;

    if let Some(email) = &data.email {
      if LocalUser::is_email_taken(context.pool(), email).await? {
        return Err(LemmyError::from_message("email_already_exists"));
      }
    }

    // We have to create both a person, and local_user

    // Register the new person
    let person_form = PersonInsertForm::builder()
      .name(username)
      .actor_id(Some(actor_id.clone()))
      .private_key(Some(actor_keypair.private_key))
      .public_key(actor_keypair.public_key)
      .inbox_url(Some(generate_inbox_url(&actor_id)?))
      .shared_inbox_url(Some(generate_shared_inbox_url(&actor_id)?))
      // If its the initial site setup, they are an admin
      .admin(Some(!local_site.site_setup))
      .instance_id(site_view.site.instance_id)
      .build();

    // insert the person
    let inserted_person = Person::create(context.pool(), &person_form)
      .await
      .map_err(|e| LemmyError::from_error_message(e, "user_already_exists"))?;

    // Automatically set their application as accepted, if they created this with open registration.
    // Also fixes a bug which allows users to log in when registrations are changed to closed.
    let accepted_application = Some(!require_registration_application);

    // Create the local user
    let local_user_form = LocalUserInsertForm::builder()
      .person_id(inserted_person.id)
      .email(data.email.as_deref().map(str::to_lowercase))
      .password_encrypted(data.password.to_string())
      .show_nsfw(Some(data.show_nsfw))
      .accepted_application(accepted_application)
      .default_listing_type(Some(local_site.default_post_listing_type))
      .build();

    let inserted_local_user = LocalUser::create(context.pool(), &local_user_form).await?;

    let mut migrate_response = MigrateResponse {
      ok: Some(true),
      user_id: Some(inserted_local_user.id),
    };

    // Log the user in directly if the site is not setup, or email verification and application aren't required
    if !local_site.site_setup {
      migrate_response.ok = Some(false);
    }

    Ok(migrate_response)
  }
}
